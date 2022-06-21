from amaranth import *
from tedium.gateware.framer.paged_fifo import PagedFIFOReadInterface
from tedium.gateware.framer.rx_fifo_to_usb import RxFIFOToUSBInAdapter

from tests.hdl import MockUSBReport, TestCase

class USBIsochronousInEndpointInterface:
    def __init__(self):
        # Input: USB SOF strobe, used to capture FIFO state
        # and determine the size of the transfer for the new
        # USB frame.
        self.start_of_frame = Signal()

        # Output: Tells USB IN endpoint how many bytes to
        # transfer in this frame. Captured at USB SOF strobe.
        self.bytes_in_frame = Signal(12)

        # Output: Data byte to be sent to the USB host.
        self.value = Signal(8)

        # Input: USB endpoint has accepted current value and
        # will expect next value to be presented at the next
        # clock.
        self.byte_advance = Signal()

class RxFIFOToUSBInAdapterTest(TestCase):
    CLOCKS = {
        "usb": 60.0e6,
    }

    def setUp_dut(self):
        m = self.m = Module()

        fifo     = self.fifo     = PagedFIFOReadInterface(10, 13)
        endpoint = self.endpoint = USBIsochronousInEndpointInterface()
        report   = self.report   = MockUSBReport()

        dut = self.dut = m.submodules.dut = RxFIFOToUSBInAdapter(fifo, endpoint, report)

    def test_stuff(self):
        self.setUp_dut()

        fifo, endpoint, report, dut = self.fifo, self.endpoint, self.report, self.dut

        with self.assertSimulation(self.m, filename="rx_fifo_to_usb_in_adapter") as sim:
            def process():
                for record_count in range(3):
                    yield dut.start_of_frame.eq(1)
                    yield fifo.r_level2.eq(record_count)
                    yield report.field_0.eq(0x69)
                    yield report.field_1.eq(0x4223)
                    yield
                    yield dut.start_of_frame.eq(0)
                    yield fifo.r_level2.eq(0)
                    # yield report.field_0.eq(0x00)
                    # yield report.field_1.eq(0x0000)

                    # Record(s)
                    for record_index in range(record_count):
                        for byte_index in range(fifo.record_size):
                            yield fifo.r_data.eq(record_index << 4 | byte_index)

                            for _ in range(byte_index // 3):
                                yield endpoint.byte_advance.eq(0)
                                yield
                            yield endpoint.byte_advance.eq(1)
                            yield
                        yield endpoint.byte_advance.eq(0)

                    # Report
                    for byte_index in range(len(report.as_value()) // 8):
                        for _ in range(byte_index):
                            yield endpoint.byte_advance.eq(0)
                            yield
                        yield endpoint.byte_advance.eq(1)
                        yield
                    yield endpoint.byte_advance.eq(0)

                    for _ in range(23):
                        yield

            sim.add_sync_process(process, domain="usb")
