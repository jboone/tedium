from amaranth import *
from tedium.gateware.framer.paged_fifo import PagedFIFOWriteInterface
from tedium.gateware.framer.tx_usb_to_fifo import TxUSBOutToFIFOAdapter, USBIsochronousOutEndpointInterface

from tests.hdl import MockUSBReport, TestCase

class TxUSBOutToFIFOAdapterTest(TestCase):
    CLOCKS = {
        "usb": 60.0e6,
    }

    def setUp_dut(self):
        m = self.m = Module()

        endpoint = self.endpoint = USBIsochronousOutEndpointInterface()
        fifo     = self.fifo     = PagedFIFOWriteInterface(10, 13)
        report   = self.report   = MockUSBReport()

        dut = self.dut = m.submodules.dut = TxUSBOutToFIFOAdapter(endpoint, fifo, report)

    def test_stuff(self):
        self.setUp_dut()

        fifo, endpoint, report, dut = self.fifo, self.endpoint, self.report, self.dut

        with self.assertSimulation(self.m, filename="tx_usb_out_to_fifo_adapter") as sim:
            def process():
                for record_count in range(3):
                    yield endpoint.start_of_frame.eq(1)
                    yield
                    yield endpoint.start_of_frame.eq(0)

                    # Report
                    for byte_index in range(len(report.as_value()) // 8):
                        yield endpoint.value.eq((record_count << 4) | (byte_index << 0))

                        for _ in range(byte_index):
                            yield endpoint.advance.eq(0)
                            yield
                        yield endpoint.advance.eq(1)
                        yield
                    yield endpoint.advance.eq(0)

                    # Record(s)
                    for record_index in range(record_count):
                        for byte_index in range(fifo.record_size):
                            yield endpoint.value.eq(record_index << 4 | byte_index)

                            for _ in range(byte_index // 3):
                                yield endpoint.advance.eq(0)
                                yield
                            yield endpoint.advance.eq(1)
                            yield
                        yield endpoint.advance.eq(0)

                    for _ in range(23):
                        yield

            sim.add_sync_process(process, domain="usb")
