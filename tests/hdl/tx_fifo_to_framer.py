from amaranth import *

from tedium.gateware.framer.paged_fifo import PagedFIFOReadInterface
from tedium.gateware.framer.timeslot import TimeslotInterface
from tedium.gateware.framer.tx_fifo_to_framer import TxFIFOToFramerAdapter

from tests.hdl import FramerTestCase, MockFrameReport

class TxFIFOToFramerAdapterTest(FramerTestCase):
    def setUp_dut(self):
        channels_count = 8
        record_size = 17
        depth_pages = 8
        
        self.setUp_timing()

        m = self.m

        self.fifo_reader = PagedFIFOReadInterface(record_size, depth_pages)
        self.timeslots = [TimeslotInterface() for _ in range(channels_count)]
        self.report = MockFrameReport()

        dut = self.dut = m.submodules.dut = TxFIFOToFramerAdapter(self.fifo_reader, self.timeslots, self.system.iface, self.report)

        m.d.comb += [
            self.serclk.eq(ClockSignal("sclk")),
            self.output_enable.eq(1),
        ]

    def test_stuff(self):
        self.setUp_dut()

        fifo, timeslots, report, dut = self.fifo_reader, self.timeslots, self.report, self.dut

        with self.assertSimulation(self.m, run_until=200e-6, filename="tx_fifo_to_framer") as sim:
            def process():
                while True:
                    addr = (yield fifo.r_addr)
                    yield fifo.r_data.eq(addr)
                    yield
                    
            sim.add_sync_process(process, domain="sync")
            