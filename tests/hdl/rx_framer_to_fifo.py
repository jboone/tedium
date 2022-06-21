import itertools

from amaranth import *
from tedium.gateware.framer.paged_fifo import PagedFIFOWriteInterface

from tedium.gateware.framer.rx_framer_to_fifo import RxFramerToFIFOAdapter
from tedium.gateware.framer.system_timing import SystemTiming
from tedium.gateware.framer.timeslot import TimeslotInterface

from tests.hdl import MockFrameReport, TestCase

class RxFramerToFIFOAdapterTest(TestCase):
    CLOCKS = {
        "sync": 18.432e6,
    }

    def setUp_dut(self):
        self.channels_count = 8
        self.timeslots_per_frame = 3

        self.report   = MockFrameReport()
        frame_record_size = self.channels_count * self.timeslots_per_frame + self.report.length_bytes()

        m = self.m = Module()

        self.fifo     = PagedFIFOWriteInterface(frame_record_size, 8)
        self.channels = [TimeslotInterface() for _ in range(self.channels_count)]
        self.system   = m.submodules.system = SystemTiming(self.timeslots_per_frame)
        self.dut      = m.submodules.dut    = RxFramerToFIFOAdapter(self.fifo, self.channels, self.system.iface, self.report)

    def test_stuff(self):
        self.setUp_dut()

        report, fifo, channels, system, dut = self.report, self.fifo, self.channels, self.system, self.dut

        clocks_per_serclk = 2
        clocks_per_timeslot = clocks_per_serclk * 8

        with self.assertSimulation(self.m, filename="rx_framer_to_fifo_adapter") as sim:
            def process():
                for _ in range(3):
                    # F bit
                    f = 1
                    mf = 1

                    for channel_n, channel in enumerate(channels):
                        yield channel.data.eq(0)
                        yield channel.f.eq(f)
                        yield channel.mf.eq(mf)

                    for n in range(clocks_per_serclk):
                        yield system.bit_end_strobe.eq(n % clocks_per_serclk == (clocks_per_serclk - 1))
                        yield
                    yield system.bit_end_strobe.eq(0)

                    for timeslot_n in range(self.timeslots_per_frame):
                        for channel_n, channel in enumerate(channels):
                            yield channel.data.eq((timeslot_n << 4) | (channel_n << 0))

                        for n in range(clocks_per_timeslot):
                            yield system.bit_end_strobe.eq(n % clocks_per_serclk == (clocks_per_serclk - 1))
                            yield
                        yield system.bit_end_strobe.eq(0)

            sim.add_sync_process(process, domain="sync")

            def process_report():
                for n in range(160):
                    if (yield system.iface.frame_strobe):
                        yield report.field_0.eq(n)
                        yield report.field_1.eq(
                              ((0x55 ^ n) << 8)
                            | ((0xaa ^ n) << 0)
                        )
                    yield

            sim.add_sync_process(process_report, domain="sync")
