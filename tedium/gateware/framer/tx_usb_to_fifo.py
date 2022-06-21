from amaranth import *

from tedium.gateware.isochronous import USBIsochronousOutEndpointTedium

from .paged_fifo import PagedFIFOWriteInterface

class USBIsochronousOutEndpointInterface:
    def __init__(self):
        self.start_of_frame = Signal()

        self.value = Signal(8)

        self.advance = Signal()

class TxUSBOutToFIFOAdapter(Elaboratable):
    """
    NOTE: `report` is updated piecemeal, byte-by-byte, at the start of the frame,
          which allows any number of frames to be transmitted afterward.
    """
    
    def __init__(self, endpoint: USBIsochronousOutEndpointTedium, fifo: PagedFIFOWriteInterface, report: Record):
        self._endpoint = endpoint
        self._fifo = fifo
        self._report = report

        self.start_of_frame = Signal()

    def elaborate(self, platform) -> Module:
        m = Module()

        endpoint, fifo = self._endpoint, self._fifo

        report_signal = self._report.as_value()
        assert(len(report_signal) % 8 == 0)

        frame_size = fifo.record_size
        report_size = len(report_signal) // 8

        report_complete_strobe = Signal()
        m.d.comb += report_complete_strobe.eq(0)

        m.d.comb += [
            fifo.w_data.eq(endpoint.value),
            fifo.w_en.eq(0),
            fifo.w_advance.eq(0),
        ]

        address = Signal(range(max(frame_size, report_size)))
        address_next = Signal.like(address)
        frame_address_last_strobe = (address == frame_size - 1) & endpoint.byte_advance
        report_address_last_strobe = (address == report_size - 1) & endpoint.byte_advance

        m.d.usb += address.eq(address_next)

        with m.FSM(domain="usb") as fsm:
            with m.State("REPORT"):
                with m.If(self.start_of_frame):
                    # If we get the USB start of frame, always reset to receiving a report.
                    m.d.comb += address_next.eq(0)
                    m.next = "REPORT"

                with m.Else():
                    # Capture the data from the endpoint in the correct byte of the report.
                    with m.If(endpoint.byte_advance):
                        m.d.usb += report_signal.word_select(address, 8).eq(endpoint.value)

                    # Advance the address only if the endpoint is advancing.
                    m.d.comb += address_next.eq(Mux(endpoint.byte_advance, address + 1, address)),

                    with m.If(report_address_last_strobe):
                        # We've read a complete report from the endpoint. Signal that fact
                        # and start receiving frame(s).
                        m.d.comb += [
                            report_complete_strobe.eq(1),
                            address_next.eq(0),
                        ]
                        m.next = "FRAME"

            with m.State("FRAME"):
                with m.If(self.start_of_frame):
                    # If we get the USB start of frame, always reset to receiving a report.
                    # If we're mid-frame, we'll bail on it without advancing, as it's probably
                    # garbage data anyway.
                    m.d.comb += address_next.eq(0)
                    m.next = "REPORT"

                with m.Else():
                    m.d.comb += [
                        fifo.w_addr.eq(address),
                        fifo.w_en.eq(endpoint.byte_advance),
                        address_next.eq(Mux(endpoint.byte_advance, address + 1, address)),
                    ]
                    with m.If(frame_address_last_strobe):
                        # Keep receiving frames. Because of the FIFO paging scheme,
                        # if the FIFO is full, we'll just overwrite the prior frame.
                        m.d.comb += [
                            fifo.w_advance.eq(1),
                            address_next.eq(0),
                        ]

        return m
