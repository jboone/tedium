from amaranth import *
from amaranth.sim.core import Settle

from tedium.gateware.framer.system_timing import SystemTiming

from tests.hdl import TestCase


class SystemTimingTest(TestCase):
    CLOCKS = {
        "sync": 18.432e6,
    }

    def setUp_dut(self):
        self.timeslots_per_frame = 3

        m = self.m = Module()
        
        dut = self.dut = m.submodules.dut = SystemTiming(self.timeslots_per_frame)

        # Advance one bit for every clock, we're in a hurry! :-)
        m.d.comb += self.dut.bit_end_strobe.eq(1)

    def test_timeslot_count(self):
        self.setUp_dut()

        dut = self.dut
        bit_strobe, iface = dut.bit_end_strobe, dut.iface
        bits_per_frame = 1 + self.timeslots_per_frame * 8

        with self.assertSimulation(self.m, filename="system_timing_timeslot_count") as sim:
            def process():
                # Allow (almost) one frame for the state to get established.
                for _ in range(bits_per_frame - 1):
                    yield

                yield Settle()
                self.assertEqual((yield iface.frame_strobe), 1)
                self.assertEqual((yield iface.timeslot_start_strobe), 0)

                for n in range(self.timeslots_per_frame):
                    yield
                    yield Settle()
                    self.assertEqual((yield iface.frame_strobe), 0)
                    self.assertEqual((yield iface.timeslot_start_strobe), 1)

                    for _ in range(7):
                        yield
                        yield Settle()
                        self.assertEqual((yield iface.frame_strobe), 0)
                        self.assertEqual((yield iface.timeslot_start_strobe), 0)

                yield
                yield Settle()
                self.assertEqual((yield iface.frame_strobe), 1)
                self.assertEqual((yield iface.timeslot_start_strobe), 0)

            sim.add_sync_process(process)
