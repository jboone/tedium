from amaranth import *

from .rx_physical import RxPhysicalInterface
from .rx_registered import RxRegistered
from .system_timing import SystemTimingInterface
from .timeslot import TimeslotInterface

class RxBlock(Elaboratable):
    """
    Combines several modules into a single framer->timeslot interface.

    `sclk` is the raw recovered clock from the receiver.

    `serclk` is the asynchronous serial clock to provide to the framer.
             It is assumed `system` timing is synchronized to this clock.

    `output_enable`: enables drivers in the FPGA, facing the framer.
                     Configure the framer signals to the correct directions
                     before asserting this.

    It is expected that the `phy` recovered clock `sclk` is processed
    elsewhere (or ignored).
    """
    def __init__(self, phy: RxPhysicalInterface, system: SystemTimingInterface):
        self._phy = phy
        self._reg = RxRegistered(self._phy)
        self._system = system

        self.sclk = Signal()
        self.serclk = Signal()
        self.output_enable = Signal()

        self.timeslot = TimeslotInterface()

    def elaborate(self, platform) -> Module:
        m = Module()

        system, timeslot, reg = self._system, self.timeslot, self._reg
        m.submodules.reg      = reg

        # Offset the frame sync provided to the framer so that the output from
        # this block is aligned with the system's timing.
        rx_f_bit_in_system_frame = system.bits_per_frame - 8
        rx_sync = Signal()
        m.d.comb += rx_sync.eq(system.bit_in_frame == rx_f_bit_in_system_frame)

        bit_strobe = system.bit_end_strobe

        timeslot_capture = system.f_bit | (system.timeslot_ls_bit ^ system.frame_last_bit)
        timeslot_strobe = system.bit_end_strobe & timeslot_capture
        f_capture = system.frame_last_bit
        frame_strobe = system.bit_end_strobe & f_capture

        ser = reg.iface.ser
        ser_shifter = Signal(8)
        ser_shifter_in = ser
        ser_shifter_shifted = Signal.like(ser_shifter)
        m.d.comb += ser_shifter_shifted.eq((ser_shifter << 1) | ser_shifter_in)
        with m.If(bit_strobe):
            m.d.sync += ser_shifter.eq(ser_shifter_shifted)

        crcsync = reg.iface.crcsync
        crcsync_shifter = Signal(8)
        crcsync_shifter_shifted = Signal.like(crcsync_shifter)
        m.d.comb += crcsync_shifter_shifted.eq((crcsync_shifter << 1) | crcsync)
        with m.If(bit_strobe):
            m.d.sync += crcsync_shifter.eq(crcsync_shifter_shifted)

        with m.If(timeslot_strobe):
            m.d.sync += timeslot.data.eq(ser_shifter_shifted)

        with m.If(frame_strobe):
            m.d.sync += [
                timeslot.f.eq(ser_shifter_shifted[-1]),
                timeslot.mf.eq(crcsync_shifter_shifted[-1]),
            ]

        m.d.comb += [
            self.sclk.eq(self._phy.sclk.i),

            # Outputs to framer.
            reg.iface.serclk.eq(self.serclk),
            reg.iface.sync.eq(rx_sync),
            reg.output_enable.eq(self.output_enable),
        ]

        return m
