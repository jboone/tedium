from amaranth import *
from amaranth.hdl.ast import Assume, Initial
from amaranth.lib.cdc import FFSynchronizer, AsyncFFSynchronizer
from amaranth.lib.coding import GrayEncoder, GrayDecoder
from amaranth.lib.fifo import FIFOInterface
from amaranth.utils import log2_int

class HackedAsyncFIFO(Elaboratable, FIFOInterface):
    __doc__ = FIFOInterface._doc_template.format(
    description="""
    JVB: HACKED-UP AsyncFIFO, exposing the memory read and write locations, so I can misuse them!

    Asynchronous first in, first out queue.

    Read and write interfaces are accessed from different clock domains, which can be set when
    constructing the FIFO.

    :class:`AsyncFIFO` can be reset from the write clock domain. When the write domain reset is
    asserted, the FIFO becomes empty. When the read domain is reset, data remains in the FIFO - the
    read domain logic should correctly handle this case.

    :class:`AsyncFIFO` only supports power of 2 depths. Unless ``exact_depth`` is specified,
    the ``depth`` parameter is rounded up to the next power of 2.
    """.strip(),
    parameters="""
    r_domain : str
        Read clock domain.
    w_domain : str
        Write clock domain.
    fwft : bool
        Always set.
    """.strip(),
    attributes="",
    r_data_valid="Valid if ``r_rdy`` is asserted.",
    r_attributes="""
    r_rst : Signal(1), out
        Asserted, for at least one read-domain clock cycle, after the FIFO has been reset by
        the write-domain reset.
    """.strip(),
    w_attributes="")

    def __init__(self, *, width, depth, r_domain="read", w_domain="write", exact_depth=False):
        if depth != 0:
            try:
                depth_bits = log2_int(depth, need_pow2=exact_depth)
                depth = 1 << depth_bits
            except ValueError:
                raise ValueError("AsyncFIFO only supports depths that are powers of 2; requested "
                                 "exact depth {} is not"
                                 .format(depth)) from None
        else:
            depth_bits = 0
        super().__init__(width=width, depth=depth, fwft=True)

        self.r_rst = Signal()
        self._r_domain = r_domain
        self._w_domain = w_domain
        self._ctr_bits = depth_bits + 1

        self.w_ptr = Signal(depth_bits)
        self.r_ptr = Signal(depth_bits)

    def elaborate(self, platform):
        m = Module()
        if self.depth == 0:
            m.d.comb += [
                self.w_rdy.eq(0),
                self.r_rdy.eq(0),
            ]
            return m

        # The design of this queue is the "style #2" from Clifford E. Cummings' paper "Simulation
        # and Synthesis Techniques for Asynchronous FIFO Design":
        # http://www.sunburst-design.com/papers/CummingsSNUG2002SJ_FIFO1.pdf

        do_write = self.w_rdy & self.w_en
        do_read  = self.r_rdy & self.r_en

        # TODO: extract this pattern into lib.cdc.GrayCounter
        produce_w_bin = Signal(self._ctr_bits)
        produce_w_nxt = Signal(self._ctr_bits)
        m.d.comb += produce_w_nxt.eq(produce_w_bin + do_write)
        m.d[self._w_domain] += produce_w_bin.eq(produce_w_nxt)

        # Note: Both read-domain counters must be reset_less (see comments below)
        consume_r_bin = Signal(self._ctr_bits, reset_less=True)
        consume_r_nxt = Signal(self._ctr_bits)
        m.d.comb += consume_r_nxt.eq(consume_r_bin + do_read)
        m.d[self._r_domain] += consume_r_bin.eq(consume_r_nxt)

        produce_w_gry = Signal(self._ctr_bits)
        produce_r_gry = Signal(self._ctr_bits)
        produce_enc = m.submodules.produce_enc = \
            GrayEncoder(self._ctr_bits)
        produce_cdc = m.submodules.produce_cdc = \
            FFSynchronizer(produce_w_gry, produce_r_gry, o_domain=self._r_domain)
        m.d.comb += produce_enc.i.eq(produce_w_nxt),
        m.d[self._w_domain] += produce_w_gry.eq(produce_enc.o)

        consume_r_gry = Signal(self._ctr_bits, reset_less=True)
        consume_w_gry = Signal(self._ctr_bits)
        consume_enc = m.submodules.consume_enc = \
            GrayEncoder(self._ctr_bits)
        consume_cdc = m.submodules.consume_cdc = \
            FFSynchronizer(consume_r_gry, consume_w_gry, o_domain=self._w_domain)
        m.d.comb += consume_enc.i.eq(consume_r_nxt)
        m.d[self._r_domain] += consume_r_gry.eq(consume_enc.o)

        consume_w_bin = Signal(self._ctr_bits)
        consume_dec = m.submodules.consume_dec = \
            GrayDecoder(self._ctr_bits)
        m.d.comb += consume_dec.i.eq(consume_w_gry),
        m.d[self._w_domain] += consume_w_bin.eq(consume_dec.o)

        produce_r_bin = Signal(self._ctr_bits)
        produce_dec = m.submodules.produce_dec = \
            GrayDecoder(self._ctr_bits)
        m.d.comb += produce_dec.i.eq(produce_r_gry),
        m.d.comb += produce_r_bin.eq(produce_dec.o)

        w_full  = Signal()
        r_empty = Signal()
        m.d.comb += [
            w_full.eq((produce_w_gry[-1]  != consume_w_gry[-1]) &
                      (produce_w_gry[-2]  != consume_w_gry[-2]) &
                      (produce_w_gry[:-2] == consume_w_gry[:-2])),
            r_empty.eq(consume_r_gry == produce_r_gry),
        ]

        m.d[self._w_domain] += self.w_level.eq((produce_w_bin - consume_w_bin))
        m.d.comb += self.r_level.eq((produce_r_bin - consume_r_bin))

        storage = Memory(width=self.width, depth=self.depth)
        w_port  = m.submodules.w_port = storage.write_port(domain=self._w_domain)
        r_port  = m.submodules.r_port = storage.read_port (domain=self._r_domain,
                                                           transparent=False)
        m.d.comb += [
            w_port.addr.eq(produce_w_bin[:-1]),
            w_port.data.eq(self.w_data),
            w_port.en.eq(do_write),
            self.w_rdy.eq(~w_full),
        ]
        m.d.comb += [
            r_port.addr.eq(consume_r_nxt[:-1]),
            self.r_data.eq(r_port.data),
            r_port.en.eq(1),
            self.r_rdy.eq(~r_empty),
        ]

        m.d.comb += [
            self.w_ptr.eq(produce_w_bin[:-1]),
            self.r_ptr.eq(consume_r_nxt[:-1]),
        ]

        # Reset handling to maintain FIFO and CDC invariants in the presence of a write-domain
        # reset.
        # There is a CDC hazard associated with resetting an async FIFO - Gray code counters which
        # are reset to 0 violate their Gray code invariant. One way to handle this is to ensure
        # that both sides of the FIFO are asynchronously reset by the same signal. We adopt a
        # slight variation on this approach - reset control rests entirely with the write domain.
        # The write domain's reset signal is used to asynchronously reset the read domain's
        # counters and force the FIFO to be empty when the write domain's reset is asserted.
        # This requires the two read domain counters to be marked as "reset_less", as they are
        # reset through another mechanism. See https://github.com/amaranth-lang/amaranth/issues/181
        # for the full discussion.
        w_rst = ResetSignal(domain=self._w_domain, allow_reset_less=True)
        r_rst = Signal()

        # Async-set-sync-release synchronizer avoids CDC hazards
        rst_cdc = m.submodules.rst_cdc = \
            AsyncFFSynchronizer(w_rst, r_rst, o_domain=self._r_domain)

        # Decode Gray code counter synchronized from write domain to overwrite binary
        # counter in read domain.
        rst_dec = m.submodules.rst_dec = \
            GrayDecoder(self._ctr_bits)
        m.d.comb += rst_dec.i.eq(produce_r_gry)
        with m.If(r_rst):
            m.d.comb += r_empty.eq(1)
            m.d[self._r_domain] += consume_r_gry.eq(produce_r_gry)
            m.d[self._r_domain] += consume_r_bin.eq(rst_dec.o)
            m.d[self._r_domain] += self.r_rst.eq(1)
        with m.Else():
            m.d[self._r_domain] += self.r_rst.eq(0)

        if platform == "formal":
            with m.If(Initial()):
                m.d.comb += Assume(produce_w_gry == (produce_w_bin ^ produce_w_bin[1:]))
                m.d.comb += Assume(consume_r_gry == (consume_r_bin ^ consume_r_bin[1:]))

        return m