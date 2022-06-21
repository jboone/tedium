from typing import List

from amaranth import *

from .paged_fifo import PagedFIFOReadInterface
from .report import Report
from .system_timing import SystemTimingInterface
from .timeslot import TimeslotInterface

class TxFIFOToFramerAdapter(Elaboratable):
    """
    Adapter is driven by system timing.
    It moves data from a paged FIFO read interface to a set of timeslot interfaces.
    It also separates out the per-frame report.
    """
    def __init__(self, fifo: PagedFIFOReadInterface, timeslots: List[TimeslotInterface], system: SystemTimingInterface, report: Report):
        self._fifo = fifo
        self._timeslots = timeslots
        self._system = system
        self._report = report

        self.report_updated_strobe = Signal()

    def elaborate(self, platform) -> Module:
        m = Module()

        fifo, timeslots, system, report = self._fifo, self._timeslots, self._system, self._report

        report_length = report.length_bytes()
        timeslots_length = len(timeslots)

        count = Signal(range(max(report_length, timeslots_length) + 1))
        count_next = Signal.like(count)
        m.d.comb += count_next.eq(count)
        m.d.sync += count.eq(count_next)

        address = Signal(range(fifo.record_size))
        address_next = Signal.like(address)
        m.d.comb += address_next.eq(address)
        m.d.sync += address.eq(address_next)

        report_count_last_strobe   = (count == report_length)
        timeslot_count_last_strobe = (count == timeslots_length)

        report_signal = report.as_value()
        m.d.comb += self.report_updated_strobe.eq(0)

        timeslots_datas = Cat([timeslot.data for timeslot in timeslots])

        with m.FSM(domain="sync") as fsm:
            with m.State("WAIT"):
                # TODO: Might be better to separate WAIT states into one for
                # frame strobe and another for timeslots. Otherwise, we could
                # potentially start in the middle of a timeslot. But practically,
                # that should never happen except for one frame, at startup.

                with m.If(system.frame_strobe):
                    m.d.comb += count_next.eq(0)
                    m.d.comb += address_next.eq(0)
                    m.next = "REPORT"
                with m.Elif(system.timeslot_ls_bit & system.bit_start_strobe):
                    m.d.comb += count_next.eq(0)
                    m.next = "TIMESLOT"

            with m.State("REPORT"):
                with m.If(report_count_last_strobe):
                    m.d.comb += self.report_updated_strobe.eq(1)
                    m.next = "WAIT"
                with m.Else():
                    m.d.comb += count_next.eq(count + 1)
                    m.d.comb += address_next.eq(address + 1)
                    m.d.sync += report_signal.word_select(count, 8).eq(fifo.r_data)

            with m.State("TIMESLOT"):
                # We must present timeslot data before the end of the LSB bit period.
                with m.If(timeslot_count_last_strobe):
                    m.next = "WAIT"
                with m.Else():
                    m.d.comb += count_next.eq(count + 1)
                    m.d.comb += address_next.eq(address + 1)
                    m.d.sync += timeslots_datas.word_select(count, 8).eq(fifo.r_data)

        m.d.comb += [
            fifo.r_addr.eq(address_next),
            fifo.r_advance.eq(system.frame_strobe),
        ]

        return m
