import itertools

from amaranth import *
from amaranth.sim.core import Delay, Settle

from tedium.gateware.framer.rx_block import RxBlock
from tedium.gateware.framer.rx_physical import RxPhysicalInterface

from tests.hdl import FramerTestCase, frame_to_bits

class RxBlockTest(FramerTestCase):
    def setUp_dut(self):
        self.setUp_timing()

        m = self.m

        phy = self.phy = RxPhysicalInterface()
        self.block = m.submodules.block = RxBlock(self.phy, self.system.iface)

        m.d.comb += [
            # Provide a clock signal on the `phy.sclk` input from the framer.
            phy.sclk.i.eq(ClockSignal("sclk")),

            # Loop the `sclk` back to the `serclk` serial clock sent to the
            # framer.
            self.serclk.eq(phy.sclk.i),
            self.block.serclk.eq(self.serclk),

            self.block.output_enable.eq(self.output_enable),
        ]

    def test_stuff(self):
        self.setUp_dut()

        phy = self.phy
        ser = phy.ser.i
        sync = phy.sync.o
        crcsync = phy.crcsync.i

        output_enable = self.output_enable

        SER_SKEW = 25e-9

        with self.assertSimulation(self.m, run_until=200e-6, filename="rx_block") as sim:
            def process_framer_rx():
                yield output_enable.eq(1)
                yield ser.eq(1)

                frames = [
                    [0, bytearray(b'\xaa\x55\xff')],
                    [1, bytearray(b'\x55\xaa\x00')],
                    [1, bytearray(b'\x66\x99\x01')],
                ]
                frames = [frame_to_bits(*frame) for frame in frames]
                frames = itertools.cycle(frames)

                crcsyncs = itertools.cycle([0, 1, 0, 0, 0, 0])
                
                frame = itertools.repeat(1)

                while True:
                    yield Settle()
                    if (yield sync):
                        frame = next(frames)
                        frame = iter(frame)

                        crcsync_value = next(crcsyncs)
                    else:
                        crcsync_value = 0

                    yield Delay(SER_SKEW)
                    value = int(next(frame))
                    yield ser.eq(value)
                    yield crcsync.eq(crcsync_value)

                    yield

            sim.add_sync_process(process_framer_rx, domain="sclk")
