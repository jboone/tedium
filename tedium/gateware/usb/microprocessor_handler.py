from amaranth import *

from luna.gateware.stream.generator import StreamSerializer
from luna.gateware.usb.stream       import USBInStreamInterface
from luna.gateware.usb.usb2.request import USBRequestHandler

from usb_protocol.types             import USBRequestType

REQUEST_REGISTER_READ     = 0
REQUEST_REGISTER_WRITE    = 1
REQUEST_FRAMER_IF_CONTROL = 2

class FramerMicroprocessorBusVendorRequest(USBRequestHandler):
    """
    NOTE: Be sure to DomainRename "sync" to "usb"!
    """
    def __init__(self):
        super(FramerMicroprocessorBusVendorRequest, self).__init__()

        self.address = Signal(15)
        self.data_wr = Signal(8)
        self.data_rd = Signal(8)
        self.write   = Signal()
        self.start   = Signal()
        self.busy    = Signal()

        self.framer_if_enable = Signal()

    def elaborate(self, platform):
        m = Module()
        interface = self.interface

        # Create convenience aliases for our interface components.
        setup               = interface.setup
        handshake_generator = interface.handshakes_out
        tx                  = interface.tx

        m.d.sync += [
            self.start.eq(0),
        ]

        m.submodules.transmitter = transmitter = \
            StreamSerializer(data_length=1, domain="usb", stream_type=USBInStreamInterface, max_length_width=1)

        with m.If(setup.type == USBRequestType.VENDOR):
            with m.FSM(domain="usb", reset="IDLE"):

                # IDLE -- not handling any active request
                with m.State('IDLE'):

                    # If we've received a new setup packet, handle it.
                    with m.If(setup.received):

                        # Select which standard packet we're going to handler.
                        with m.Switch(setup.request):

                            with m.Case(REQUEST_REGISTER_READ):
                                m.d.sync += [
                                    self.address.eq(setup.index[:15]),
                                    self.write.eq(0),
                                    self.start.eq(1),
                                ]
                                m.next = 'REG_READ'

                            with m.Case(REQUEST_REGISTER_WRITE):
                                m.d.sync += [
                                    self.address.eq(setup.index[:15]),
                                    self.data_wr.eq(setup.value[:8]),
                                    self.write.eq(1),
                                    self.start.eq(1),
                                ]
                                m.next = 'REG_WRITE'

                            with m.Case(REQUEST_FRAMER_IF_CONTROL):
                                m.d.sync += [
                                    self.framer_if_enable.eq(setup.value[0]),
                                ]

                                m.next = 'FRAMER_IF_CONTROL'

                            with m.Case():
                                m.next = 'UNHANDLED'

                with m.State('REG_READ'):
                    # Wait for framer transaction to complete.

                    # Connect our transmitter up to the output stream...
                    m.d.comb += [
                        transmitter.stream          .attach(self.interface.tx),
                        Cat(transmitter.data[0:1])  .eq(self.data_rd),
                        transmitter.max_length      .eq(1)
                    ]

                    # ... trigger it to respond when data's requested...
                    with m.If(self.interface.data_requested):
                        m.d.comb += [
                            self.interface.handshakes_out.nak.eq(self.busy),
                            transmitter.start.eq(~self.busy),
                        ]

                    # ... and ACK our status stage.
                    with m.If(self.interface.status_requested):
                        m.d.comb += self.interface.handshakes_out.ack.eq(1)
                        m.next = 'IDLE'

                with m.State('REG_WRITE'):
                    # Wait for framer transaction to complete.

                    # Provide an response to the STATUS stage.
                    with m.If(self.interface.status_requested):

                        # If our stall condition is met, stall; otherwise, send a ZLP [USB 8.5.3].
                        stall_condition = 0
                        with m.If(stall_condition):
                            m.d.comb += self.interface.handshakes_out.stall.eq(1)
                        with m.Elif(self.busy):
                            m.d.comb += self.interface.handshakes_out.nak.eq(1)
                        with m.Else():
                            m.d.comb += self.send_zlp()

                    # Accept the relevant value after the packet is ACK'd...
                    with m.If(self.interface.handshakes_in.ack):
                        # m.d.comb += [
                        # 	write_strobe      .eq(1),
                        # 	new_value_signal  .eq(self.interface.setup.value[0:7])
                        # ]

                        # ... and then return to idle.
                        m.next = 'IDLE'

                with m.State('FRAMER_IF_CONTROL'):
                    
                    # Provide an response to the STATUS stage.
                    with m.If(self.interface.status_requested):

                        # If our stall condition is met, stall; otherwise, send a ZLP [USB 8.5.3].
                        with m.If(stall_condition):
                            m.d.comb += self.interface.handshakes_out.stall.eq(1)
                        with m.Else():
                            m.d.comb += self.send_zlp()

                    # Accept the relevant value after the packet is ACK'd...
                    with m.If(self.interface.handshakes_in.ack):

                        # ... and then return to idle.
                        m.next = 'IDLE'

                # UNHANDLED -- we've received a request we're not prepared to handle
                with m.State('UNHANDLED'):

                    # When we next have an opportunity to stall, do so,
                    # and then return to idle.
                    with m.If(interface.status_requested | interface.data_requested):
                        m.d.comb += interface.handshakes_out.stall.eq(1)
                        m.next = 'IDLE'

        return m
