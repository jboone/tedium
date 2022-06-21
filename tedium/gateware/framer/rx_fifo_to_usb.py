from amaranth import *

from ..usb.isochronous import USBIsochronousInEndpointTedium

from .paged_fifo import PagedFIFOReadInterface
from .report import Report

class RxFIFOToUSBInAdapter(Elaboratable):
    """
    Thing to connect FIFO to USB IN endpoint

    FIFO: Zero, one, or two pages of the FIFO are read depending on how much
          data is available in the FIFO for reading.
    
    Endpoint: USB IN endpoint and SOF strobe.

    Report: Will be sent at the end of the USB packet. Expected to be stable
            except during the SOF strobe. So be sure to capture values that
            may change during the USB frame.

    NOTE: I'm unsure how well multiple packets per frame will be handled.
    """

    def __init__(self, fifo: PagedFIFOReadInterface, endpoint: USBIsochronousInEndpointTedium, report: Report):
        self._fifo = fifo
        self._endpoint = endpoint
        self._report = report

        self.start_of_frame = Signal()

    def elaborate(self, platform) -> Module:
        m = Module()

        fifo, endpoint = self._fifo, self._endpoint

        report_signal = self._report.as_value()
        report_size = self._report.length_bytes()

        frame_size = fifo.record_size

        # Expose the current state of the FIFO.
        # The USB IN endpoint will capture it at the start of
        # a USB (micro)frame.
        pages_in_frame = Signal(2)
        bytes_in_usb_frame = Signal.like(endpoint.bytes_in_frame)
        with m.Switch(fifo.r_level2):
            with m.Case(0):
                m.d.comb += [
                    pages_in_frame.eq(0),
                    bytes_in_usb_frame.eq(0 * frame_size + report_size),
                ]
            with m.Case(1):
                m.d.comb += [
                    pages_in_frame.eq(1),
                    bytes_in_usb_frame.eq(1 * frame_size + report_size),
                ]
            with m.Default():
                m.d.comb += [
                    pages_in_frame.eq(2),
                    bytes_in_usb_frame.eq(2 * frame_size + report_size),
                ]
        

        m.d.comb += [
            endpoint.bytes_in_frame.eq(bytes_in_usb_frame),
        ]

        m.d.comb += [
            fifo.r_advance.eq(0),
        ]

        address = Signal(range(max(frame_size, report_size)))
        address_next = Signal.like(address)
        frame_address_last_strobe = (address == frame_size - 1) & endpoint.byte_advance
        report_address_last_strobe = (address == report_size - 1) & endpoint.byte_advance

        m.d.usb += address.eq(address_next)

        with m.FSM(domain="usb") as fsm:
            with m.State("WAIT_SOF"):
                with m.If(self.start_of_frame):
                    with m.Switch(pages_in_frame):
                        with m.Case(0):
                            m.next = "REPORT"
                        with m.Case(1):
                            m.next = "FRAME_1"
                        with m.Default():
                            m.next = "FRAME_2"

            with m.State("FRAME_2"):
                m.d.comb += [
                    endpoint.value.eq(fifo.r_data),
                    address_next.eq(Mux(endpoint.byte_advance, address + 1, address)),
                    fifo.r_addr.eq(address_next),
                ]
                with m.If(frame_address_last_strobe):
                    m.d.comb += [
                        fifo.r_advance.eq(1),
                        address_next.eq(0),
                    ]
                    m.next = "FRAME_1"

            with m.State("FRAME_1"):
                m.d.comb += [
                    endpoint.value.eq(fifo.r_data),
                    address_next.eq(Mux(endpoint.byte_advance, address + 1, address)),
                    fifo.r_addr.eq(address_next),
                ]
                with m.If(frame_address_last_strobe):
                    m.d.comb += [
                        fifo.r_advance.eq(1),
                        address_next.eq(0),
                    ]
                    m.next = "REPORT"

            with m.State("REPORT"):
                m.d.comb += [
                    endpoint.value.eq(report_signal.word_select(address, 8)),
                    address_next.eq(Mux(endpoint.byte_advance, address + 1, address)),
                ]
                with m.If(report_address_last_strobe):
                    m.d.comb += [
                        address_next.eq(0),
                    ]
                    m.next = "WAIT_SOF"
                
        return m
