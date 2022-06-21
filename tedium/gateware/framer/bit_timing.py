from amaranth import *
from amaranth.hdl.ast import Fell
from amaranth.lib.cdc import FFSynchronizer

class BitTimingInterface:
    def __init__(self):
        self.bit_end_strobe = Signal()
        self.bit_start_strobe = Signal()

class BitTiming(Elaboratable):
    """
    Produce `sync`-domain pulse for each serial clock cycle.
    Due to two-stage synchronization and synchronous capture of
    the falling-edge detection, the resulting pulse is
    approximately 2-3 `sync` clock cycles after the falling edge
    of `serclk`. This determines the low bound for `sync` frequency
    relative to the `serclk` frequency.
    """

    def __init__(self):
        self.serclk = Signal()

        self.iface = BitTimingInterface()

    def elaborate(self, platform) -> Module:
        m = Module()

        serclk, iface = self.serclk, self.iface

        serclk_sync = Signal()
        m.submodules.ff_sync = FFSynchronizer(i=serclk, o=serclk_sync, o_domain="sync")
        m.d.comb += iface.bit_end_strobe.eq(Fell(serclk_sync, domain="sync"))
        m.d.sync += iface.bit_start_strobe.eq(iface.bit_end_strobe)

        return m
