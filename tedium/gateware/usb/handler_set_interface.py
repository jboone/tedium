from amaranth import *

from luna.gateware.usb.usb2.request import USBRequestHandler

from usb_protocol.types import USBRequestType, USBRequestRecipient, USBStandardRequests

from .descriptors_vendor import Descriptors

class SetInterfaceRequestHandler(USBRequestHandler):
    """ Support SET_INTERFACE requests """

    def __init__(self):
        super().__init__()

        self.frame_stream_interface_altsetting         = Signal()

    def elaborate(self, platform):
        m = Module()

        interface         = self.interface
        setup             = self.interface.setup

        with m.If(setup.type == USBRequestType.STANDARD):
            with m.If((setup.recipient == USBRequestRecipient.INTERFACE) &
                      (setup.request == USBStandardRequests.SET_INTERFACE)):

                interface_nr   = setup.index
                alt_setting_nr = setup.value

                with m.If(interface.status_requested):
                    m.d.comb += self.send_zlp()

                with m.If(interface.handshakes_in.ack):
                    with m.Switch(interface_nr):
                        if hasattr(Descriptors.InterfaceNumber, 'FrameStream'):
                            with m.Case(Descriptors.InterfaceNumber.FrameStream):
                                m.d.usb += self.frame_stream_interface_altsetting.eq(alt_setting_nr)

        return m
