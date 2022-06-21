from amaranth import *

from .system_timing import SystemTimingInterface
from .timeslot import TimeslotInterface
from .tx_physical import TxPhysicalInterface
from .tx_registered import TxRegistered

class TxBlock(Elaboratable):
    def __init__(self, phy: TxPhysicalInterface, system: SystemTimingInterface):
        self._phy = phy
        self._reg = TxRegistered(self._phy)
        self._system = system

        # Input: Timeslot data and... frame/multiframe flag data. Maybe I should
        # rename it...
        self.timeslot = TimeslotInterface()

        self.serclk = Signal()
        self.output_enable = Signal()

    def elaborate(self, platform) -> Module:
        m = Module()

        timeslot, system, reg = self.timeslot, self._system, self._reg

        m.submodules.reg = reg

        # Timeslot serialization incurs a delay. Also delay the frame and multiframe
        # sync pulses to match.
        delay_clocks = 8
        tx_f_bit_in_system_frame = delay_clocks
        tx_sync = (system.bit_in_frame == tx_f_bit_in_system_frame)

        # This seems to work by dumb luck! But in thinking more about it,
        # This fills the LSB on the first shift of the loaded value, so after
        # the MSB is provided. In the case of the last timeslot, the value
        # is shifted out during the F bit and first seven timeslot bits of the
        # next frame. And `iface.f` has its new value for the new frame by
        # then.
        ser_shifter_in = timeslot.f

        ser_shifter = Signal(8)
        ser_shifter_out = ser_shifter[-1]
        ser_shifter_shifted = Signal.like(ser_shifter)
        m.d.comb += ser_shifter_shifted.eq((ser_shifter << 1) | ser_shifter_in)
        with m.If(system.timeslot_ls_bit & system.bit_end_strobe):
            m.d.sync += ser_shifter.eq(timeslot.data)
        with m.Elif( system.bit_end_strobe):
            m.d.sync += ser_shifter.eq(ser_shifter_shifted)

        m.d.comb += [
            # Outputs to framer.
            reg.iface.serclk.eq(self.serclk),
            reg.iface.ser.eq(ser_shifter_out),
            reg.iface.sync.eq(tx_sync),
            reg.iface.msync.eq(tx_sync & timeslot.mf),
            reg.output_enable.eq(self.output_enable),
        ]

        return m
