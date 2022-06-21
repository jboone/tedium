from amaranth import *

class SystemTimingInterface:
    def __init__(self, timeslots_per_frame: int):
        self.timeslots_per_frame = timeslots_per_frame
        self.bits_in_timeslot = 8   # NOTE: You can't really change this, there's several assumptions in the code that would be broken.
        self.bits_per_frame = 1 + (self.timeslots_per_frame * self.bits_in_timeslot)

        # `sync`-length signals (strobes)
        self.bit_end_strobe = Signal()
        self.bit_start_strobe = Signal()
        self.timeslot_end_strobe = Signal()
        self.timeslot_start_strobe = Signal()
        self.frame_strobe = Signal()

        # Bit-length signals (bit)
        self.f_bit = Signal()
        self.timeslot_ms_bit = Signal()
        self.timeslot_ls_bit = Signal()
        self.frame_last_bit = Signal()
        self.bit_in_frame = Signal(range(self.bits_per_frame))
        self.bit_in_timeslot = Signal(range(self.bits_in_timeslot))

        # Timeslot-length signals
        self.timeslot_in_frame = Signal(range(timeslots_per_frame))
        self.timeslot_first = Signal()
        self.timeslot_last = Signal()

        # Clock-length signals
        self.clock_count = Signal(8)

class SystemTiming(Elaboratable):
    """
    Keeps track of position within frame for the entire system. All
    other entities use this position within the frame to calculate
    their own offset position in time, to bring everything into
    alignment with the system timing.
    """
    def __init__(self, timeslots_per_frame: int):
        # Input: end of a bit-time, causes values for a new bit-time
        # to appear after the next clock edge.
        self.bit_end_strobe = Signal()

        self.iface = SystemTimingInterface(timeslots_per_frame)

    def elaborate(self, platform) -> Module:
        m = Module()

        iface = self.iface

        timeslot_count_last = iface.timeslots_per_frame - 1
        bit_count_last = iface.bits_per_frame - 1

        bit_count = Signal(range(iface.bits_per_frame))
        bit_count_next = Signal.like(bit_count)
        m.d.comb += bit_count_next.eq(Mux(bit_count == bit_count_last, 0, bit_count + 1))

        bit_in_timeslot = Signal.like(iface.bit_in_timeslot)
        bit_in_timeslot_next = Signal.like(bit_in_timeslot)
        m.d.comb += bit_in_timeslot_next.eq(Mux(iface.frame_last_bit, 0, (bit_count & 7) ^ 7))

        clock_count = Signal.like(iface.clock_count)
        clock_count_next = Signal.like(clock_count)
        m.d.comb += clock_count_next.eq(Mux(self.bit_end_strobe, 0, clock_count + 1))

        f_bit_next = (bit_count_next == 0)
        timeslot_ms_bit_next = ((bit_count & 7) == 0) & ~f_bit_next
        timeslot_ls_bit_next = (bit_count & 7) == 7
        frame_last_bit_next = bit_count == (bit_count_last - 1)
        timeslot_in_frame_next = Mux(iface.frame_last_bit, 0, bit_count >> 3)

        with m.If(self.bit_end_strobe):
            m.d.sync += [
                bit_count.eq(bit_count_next),
                bit_in_timeslot.eq(bit_in_timeslot_next),
                iface.f_bit.eq(f_bit_next),
                iface.timeslot_ls_bit.eq(timeslot_ls_bit_next),
                iface.timeslot_ms_bit.eq(timeslot_ms_bit_next),
                iface.frame_last_bit.eq(frame_last_bit_next),
                iface.timeslot_in_frame.eq(timeslot_in_frame_next),
            ]

        m.d.sync += [
            iface.timeslot_start_strobe.eq(timeslot_ms_bit_next & self.bit_end_strobe),
            iface.frame_strobe.eq(f_bit_next & self.bit_end_strobe),
            iface.bit_start_strobe.eq(self.bit_end_strobe),
            clock_count.eq(clock_count_next),
        ]

        m.d.comb += [
            iface.timeslot_end_strobe.eq(iface.timeslot_ls_bit & self.bit_end_strobe),
            iface.bit_in_frame.eq(bit_count),
            iface.bit_in_timeslot.eq(bit_in_timeslot),
            iface.bit_end_strobe.eq(self.bit_end_strobe),
            iface.clock_count.eq(clock_count),
        ]

        with m.If(self.bit_end_strobe):
            with m.If(timeslot_ms_bit_next):
                m.d.sync += [
                    iface.timeslot_first.eq(timeslot_in_frame_next == 0),
                    iface.timeslot_last.eq(timeslot_in_frame_next == timeslot_count_last),
                ]
            with m.Elif(f_bit_next):
                m.d.sync += iface.timeslot_last.eq(timeslot_in_frame_next == timeslot_count_last)
        
        return m
