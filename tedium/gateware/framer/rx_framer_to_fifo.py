from typing import List

from amaranth import *

from .paged_fifo import PagedFIFOWriteInterface
from .report import Report
from .system_timing import SystemTimingInterface
from .timeslot import TimeslotInterface

class RxFramerToFIFOAdapter(Elaboratable):
    """
    Interleave timeslot data from one or more `TimeslotInterface`s.
    Append a `Report`.
    Sequence data into a `PagedFIFOWriteInterface`.
    Timing is provided by a `SystemTimingInterface`.
    """

    def __init__(self, fifo: PagedFIFOWriteInterface, timeslots: List[TimeslotInterface], system: SystemTimingInterface, report: Report):
        self._fifo = fifo
        self._timeslots = timeslots
        self._system = system
        self._report = report

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

        m.d.comb += fifo.w_en.eq(0)

        report_signal = report.as_value()

        timeslots_datas = Cat([timeslot.data for timeslot in timeslots])

        with m.FSM(domain="sync") as fsm:
            with m.State("WAIT"):
                with m.If(system.timeslot_start_strobe):
                    with m.If(system.timeslot_first):
                        m.d.comb += address_next.eq(0)
                    m.d.comb += count_next.eq(0)
                    m.next = "TIMESLOT"

            with m.State("TIMESLOT"):
                with m.If(timeslot_count_last_strobe):
                    with m.If(system.timeslot_last):
                        m.d.comb += count_next.eq(0)
                        m.next = "REPORT"
                    with m.Else():
                        m.next = "WAIT"
                with m.Else():
                    m.d.comb += count_next.eq(count + 1)
                    m.d.comb += address_next.eq(address + 1)
                    m.d.comb += fifo.w_data.eq(timeslots_datas.word_select(count, 8))
                    m.d.comb += fifo.w_en.eq(1)

            with m.State("REPORT"):
                with m.If(report_count_last_strobe):
                    m.next = "WAIT"
                with m.Else():
                    m.d.comb += count_next.eq(count + 1)
                    m.d.comb += address_next.eq(address + 1)
                    m.d.comb += fifo.w_data.eq(report_signal.word_select(count, 8))
                    m.d.comb += fifo.w_en.eq(1)

        m.d.comb += [
            fifo.w_addr.eq(address),
            fifo.w_advance.eq(system.frame_strobe),
        ]

        return m
