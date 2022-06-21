import itertools

from amaranth import *
from amaranth.sim.core import Settle

from tedium.gateware.framer.tx_block import TxBlock
from tedium.gateware.framer.tx_physical import TxPhysicalInterface

from tests.hdl import FramerTestCase

class TxBlockTest(FramerTestCase):
    def setUp_dut(self):
        self.setUp_timing()

        m = self.m

        self.phy = TxPhysicalInterface()
        self.block = m.submodules.block = TxBlock(self.phy, self.system.iface)

        m.d.comb += [
            # Use the simulation `sclk` as our serial clock.
            self.serclk.eq(ClockSignal("sclk")),

            # Provide a clock to `serclk`, which is sent to the framer for
            # use in capturing the other signals on the interface to the framer.
            self.block.serclk.eq(self.serclk),

            self.block.output_enable.eq(self.output_enable),
        ]

    def test_stuff(self):
        self.setUp_dut()

        output_enable = self.output_enable

        with self.assertSimulation(self.m, run_until=200e-6, filename="tx_block") as sim:
            system, ts = self.system, self.block.timeslot
            system = system.iface

            def process_timeslot_tx():
                frames = [
                    [0, bytearray(b'\x80\x55\xff')],
                    [1, bytearray(b'\x7f\xaa\x00')],
                    [1, bytearray(b'\xcc\x99\x01')],
                ]
                frames = itertools.cycle(frames)

                mfs = itertools.cycle([0, 1, 0, 0, 0, 0])

                frame = itertools.repeat(1)
                f = 0

                yield output_enable.eq(1)

                while True:
                    yield Settle()

                    if (yield system.frame_strobe):
                        frame = next(frames)
                        f, frame = frame
                        mf = next(mfs)
                        frame = iter(frame)
                        yield ts.f.eq(f)
                        yield ts.mf.eq(mf)

                    if (yield system.timeslot_start_strobe):
                        value = next(frame)
                        yield ts.data.eq(value)

                    yield

            sim.add_sync_process(process_timeslot_tx, domain="sync")