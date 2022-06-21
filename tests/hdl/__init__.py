from contextlib import contextmanager
import unittest

from amaranth import *
from amaranth.sim.core import Simulator

from tedium.gateware.framer.bit_timing import BitTiming
from tedium.gateware.framer.report import Report
from tedium.gateware.framer.system_timing import SystemTiming

class TestCase(unittest.TestCase):
    CLOCKS = {}
        
    @contextmanager
    def assertSimulation(self, module: Module, run_until=None, filename="test"):
        sim = Simulator(module)

        for domain, frequency in self.CLOCKS.items():
            sim.add_clock(1.0 / frequency, domain=domain)

        self.traces = []

        yield sim
        
        with sim.write_vcd(f"{filename}.vcd", traces=self.traces):
            if run_until:
                sim.run_until(run_until)
            else:
                sim.run()

class FramerTestCase(TestCase):
    CLOCKS = {
        "sclk": 1.544e6,
        "sync": 18.432e6,
    }

    def setUp_timing(self):
        self.timeslots_per_frame = 3

        m = self.m = Module()

        m.domains.sync = ClockDomain()
        m.domains.sclk = ClockDomain()

        self.serclk = Signal()
        self.output_enable = Signal()

        bit = self.bit = m.submodules.bit = BitTiming()
        m.d.comb += bit.serclk.eq(self.serclk)

        system = self.system = m.submodules.system = SystemTiming(self.timeslots_per_frame)
        m.d.comb += system.bit_end_strobe.eq(bit.iface.bit_end_strobe)

class MockFrameReport(Report):
    LAYOUT = [
        ('field_0', 8),
        ('field_1', 16),
    ]

    def __init__(self):
        super().__init__(self.LAYOUT, name=self.__class__.__name__)

class MockUSBReport(Report):
    LAYOUT = [
        ('field_0', 8),
        ('field_1', 16),
    ]

    def __init__(self):
        super().__init__(self.LAYOUT, name=self.__class__.__name__)

def frame_to_bits(f: bool, timeslots: bytearray) -> str:
    result = ['1' if f else '0']
    for value in timeslots:
        result.append(f"{value:08b}")
    return ''.join(result)
