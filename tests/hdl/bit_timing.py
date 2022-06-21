from amaranth import *
from amaranth.sim.core import Settle

from tedium.gateware.framer.bit_timing import BitTiming

from tests.hdl import TestCase

class BitTimingTest(TestCase):
    CLOCKS = {
        "serclk": 1.544e6,
        "sync": 18.432e6,
    }

    def setUp_dut(self):
        self.m = Module()
        # self.clock_sel = ClockSelectInterface(1)
        self.dut = self.m.submodules.dut = BitTiming()

        self.m.d.comb += [
            # self.clock_sel.selection_index.eq(0),
            self.dut.serclk.eq(ClockSignal("serclk")),
        ]

    def test_strobe(self):
        self.setUp_dut()

        serclk = self.dut.serclk
        iface = self.dut.iface

        with self.assertSimulation(self.m, filename="bit_timing_strobe") as sim:
            def process():
                for _ in range(100):

                    # Wait for rising edge of `serclk`.
                    while (yield serclk) == False:
                        yield

                    # Wait for falling edge of `serclk`.
                    while (yield serclk) == True:
                        yield

                    yield Settle()
                    self.assertEqual((yield iface.bit_end_strobe), 0)
                    yield

                    # yield Settle()
                    # self.assertEqual((yield iface.bit_end_strobe), 0)
                    # yield

                    yield Settle()
                    self.assertEqual((yield iface.bit_end_strobe), 1)
                    yield

                    yield Settle()
                    self.assertEqual((yield iface.bit_end_strobe), 0)

            sim.add_sync_process(process)
