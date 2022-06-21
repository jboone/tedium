from amaranth import *
from amaranth.sim.core import Delay, Settle
from tedium.gateware.framer.rx_physical import RxPhysicalInterface
from tedium.gateware.framer.rx_registered import RxRegistered

from tests.hdl import TestCase

class RxPhysicalTest(TestCase):
    def setUp_xxx(self):
        m = self.m = Module()

        sclk = self.sclk = Signal()
        m.domains.sclk = ClockDomain()
        m.d.comb += ClockSignal("sclk").eq(sclk)

        self.phy = RxPhysicalInterface()
        dut = self.dut = m.submodules.dut = RxRegistered(self.phy)

        m.d.comb += [
            dut.phy.sclk.i.eq(sclk),
            dut.iface.serclk.eq(sclk),
        ]

    def test_output_enable(self):
        self.setUp_xxx()

        dut = self.dut
        phy, reg = dut.phy, dut.iface

        with self.assertSimulation(self.m) as sim:
            def process():
                yield dut.output_enable.eq(0)
                yield Settle()

                self.assertEqual((yield phy.serclk.oe), 0)
                self.assertEqual((yield phy.sync.oe), 0)

                yield dut.output_enable.eq(1)
                yield Settle()
                self.assertEqual((yield phy.serclk.oe), 1)
                self.assertEqual((yield phy.sync.oe), 1)

            sim.add_process(process)

    def test_launch(self):
        self.setUp_xxx()
        
        dut = self.dut
        phy, reg = dut.phy, dut.iface
        serclk = reg.serclk

        half_period = 1 / 1.544e6 / 2

        SETUP_TIME = 20e-9

        with self.assertSimulation(self.m) as sim:
            sequence = [
                0, 1, 0, 0, 1, 1, 0,
            ]

            def process():
                for value in sequence:
                    yield serclk.eq(0)
                    yield Delay(half_period - SETUP_TIME)

                    yield reg.sync.eq(value)

                    yield Delay(SETUP_TIME)

                    yield serclk.eq(1)
                    yield Settle()

                    self.assertEqual((yield phy.sync.o), value)

                    yield Delay(half_period)

            sim.add_process(process)

    def test_capture(self):
        self.setUp_xxx()
        
        dut = self.dut
        phy, reg = dut.phy, dut.iface
        serclk = reg.serclk

        half_period = 1 / 1.544e6 / 2

        SETUP_TIME = 20e-9

        with self.assertSimulation(self.m) as sim:
            sequence = [
                (0, 0, 0),
                (1, 0, 0),
                (0, 1, 0),
                (0, 0, 1),
                (1, 0, 1),
                (1, 1, 1),
                (0, 0, 0),
            ]
            
            def process():
                for values in sequence:
                    yield serclk.eq(1)
                    yield Delay(half_period - SETUP_TIME)

                    yield phy.ser.i.eq(    values[0])
                    yield phy.crcsync.i.eq(values[1])
                    yield phy.casync.i.eq( values[2])

                    yield Delay(SETUP_TIME)

                    yield serclk.eq(0)
                    yield Settle()

                    self.assertEqual((yield reg.ser),     values[0])
                    self.assertEqual((yield reg.crcsync), values[1])
                    self.assertEqual((yield reg.casync),  values[2])

                    yield Delay(half_period)

            sim.add_process(process)
