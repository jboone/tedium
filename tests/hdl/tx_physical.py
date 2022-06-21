from amaranth import *
from amaranth.sim.core import Delay, Settle, Tick
from tedium.gateware.framer.tx_physical import TxPhysicalInterface
from tedium.gateware.framer.tx_registered import TxRegistered

from tests.hdl import TestCase

class TxPhysicalTest(TestCase):
    def setUp_xxx(self):
        m = self.m = Module()

        serclk = self.serclk = Signal()
        m.domains.serclk = ClockDomain()
        m.d.comb += ClockSignal("serclk").eq(serclk)

        self.phy = TxPhysicalInterface()
        dut = self.dut = m.submodules.dut = TxRegistered(self.phy)

        m.d.comb += dut.iface.serclk.eq(serclk)

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
                self.assertEqual((yield phy.msync.oe), 0)

                yield dut.output_enable.eq(1)
                yield Settle()
                self.assertEqual((yield phy.serclk.oe), 1)
                self.assertEqual((yield phy.sync.oe), 1)
                self.assertEqual((yield phy.msync.oe), 1)

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
                    yield serclk.eq(0)
                    yield Delay(half_period - SETUP_TIME)

                    yield reg.ser.eq(values[0])
                    yield reg.sync.eq(values[1])
                    yield reg.msync.eq(values[2])

                    yield Delay(SETUP_TIME)

                    yield serclk.eq(1)
                    yield Settle()

                    self.assertEqual((yield phy.ser.o), values[0])
                    self.assertEqual((yield phy.sync.o), values[1])
                    self.assertEqual((yield phy.msync.o), values[2])

                    yield Delay(half_period)

            sim.add_process(process)
