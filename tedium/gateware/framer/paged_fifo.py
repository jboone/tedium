from amaranth import *
from amaranth.utils import bits_for, log2_int

from .hacked_fifo import HackedAsyncFIFO

# Hack to address simulation of memories larger than 3K words:
# https://github.com/amaranth-lang/amaranth/issues/359
# import sys
# sys.setrecursionlimit(4096)

class PagedFIFOWriteInterface:
    """
    * `w_addr` address within page to write to.
    * `w_data` data to write.
    * `w_en`   enable writing at the next clock edge
    * `w_level` number of in-use pages in FIFO, delayed by... TODO: two clocks, I think?
    * `w_full` is asserted when there's no empty page in the FIFO.
    * `w_advance` move to the next page in the FIFO, if there's an empty page available.
                  If not, the page pointer remains at the current page.
    * `w_overflow` strobes during `w_advance` if there's no free page available (the FIFO
                   is full).
    """
    def __init__(self, record_size: int, depth_pages: int):
        self.w_addr    = Signal(range(record_size))
        self.w_data    = Signal(8)
        self.w_en      = Signal()
        self.w_level2  = Signal(range(depth_pages))
        self.w_full    = Signal()
        self.w_advance = Signal()
        self.w_overflow = Signal()  # Strobes if advance attempted but FIFO isn't ready (FIFO is full).

        # How large are the records being written to the FIFO page?
        self.record_size = record_size

class PagedFIFOReadInterface:
    """
    * `r_addr` address within the page to read from.
    * `r_data` data read from FIFO from the last cycle's `r_addr` value.
    * `r_level` number of in-use pages in FIFO, delayed by... TODO: two clocks, I think?
    * `r_empty` is asserted when there's no written page in the FIFO.
    * `r_advance` move to the next in-use page in the FIFO, if any. If not, the page
                  pointer is unchanged.
    * `r_underflow` strobes during `r_advance` if there's no written page in the FIFO
                    (the FIFO is empty).
    """
    def __init__(self, record_size: int, depth_pages: int):
        self.r_addr    = Signal(range(record_size))
        self.r_data    = Signal(8)
        self.r_level2  = Signal(range(depth_pages))
        self.r_empty   = Signal()

        # `r_advance` moves to the next page, if there is a page available in the FIFO. Otherwise,
        # the page is unchanged.
        self.r_advance = Signal()
        self.r_underflow = Signal()

        # How large are the records being read from the FIFO page?
        self.record_size = record_size

class PagedAsyncFIFO(Elaboratable):
    """
    Paged FIFO. Adancing the read or write pointer advances the memory page visible
    through the memory-like interface.
    
    An overflow strobe will be issued if the writer attempts to advance the page to
    the page being read, and the current write page will not change (further writes
    will occur to the same page as before the advance was attempted).

    Similarly, an underflow strobe will be issued if the reader attempts to advance
    the page to the page being written, and the current read page will not change
    (further reads will occur to the same page as before the advance was attempted).
    """
    # def __init__(self, writer: PagedFIFOWriteInterface, reader: PagedFIFOReadInterface, depth_pages: int, r_domain: str, w_domain: str):
    def __init__(self, record_size: int, depth_pages: int, r_domain: str, w_domain: str):
        # assert(writer.record_size == reader.record_size)

        # If a non-power-of-two `depth_pages` is specified, the AsyncFIFO will force the depth
        # to be a power of two, but the Memory() will be the exact depth specified. So disallow
        # this situation for now.
        assert(log2_int(depth_pages))

        # self.writer = writer
        # self.reader = reader
        self.writer = PagedFIFOWriteInterface(record_size, depth_pages)
        self.reader = PagedFIFOReadInterface(record_size, depth_pages)

        # self._record_size = self.reader.record_size
        self._record_size = record_size
        self._bits_for_record_size = bits_for(self._record_size - 1)
        self._page_size = 1 << self._bits_for_record_size
        self._depth_pages = depth_pages
        self._r_domain = r_domain
        self._w_domain = w_domain

        # TODO: It's kinda gross, truncating the addresses to the correct width
        # later in the code.
        # assert(len(self.writer.w_addr) >= self._bits_for_record_size)
        # assert(len(self.reader.r_addr) >= self._bits_for_record_size)

    def elaborate(self, platform) -> Module:
        m = Module()

        # A dastardly trick, where we borrow the counting mechanism of another
        # FIFO mechanism, but never read or write it.
        m.submodules.fifo = fifo = HackedAsyncFIFO(
            width=8, depth=self._depth_pages, r_domain=self._r_domain, w_domain=self._w_domain
        )
        # assert(len(self.reader.r_level) >= len(fifo.r_level))
        # assert(len(self.writer.w_level) >= len(fifo.w_level))

        # In order to not be reading the page being written, or write the
        # page being read, we need to trim the minimum and maximum levels
        # by one count each.
        # 
        # Evidence of correctness on the read interface is seen when reading
        # from the FIFO until empty, and the last written page is repeated
        # when reading the empty FIFO repeatedly.
        #
        # Evidence of correctness on the write interface is a bit harder
        # to come by... I'll need to think on this. Ah, how about this...
        # Take a new FIFO. Write to the FIFO until full, and then continue to
        # write to it with new values. Meanwhile, read from the reader
        # interface, which is resting on the first page. 
        fifo_level_empty = 1
        fifo_level_full = fifo.depth - 1

        assert(fifo.depth == self._depth_pages)

        # TODO: Block reads if not ready? I guess the AsyncFIFO already does that...

        memory = Memory(width=8, depth=self._depth_pages * self._page_size)
        m.submodules.w_port = w_port = memory.write_port(domain=self._w_domain)
        m.submodules.r_port = r_port = memory.read_port( domain=self._r_domain,
                                                         transparent=False)

        # Delay the page pointer from the FIFO by one clock so that we can 
        # advance on the last read address of a page and start reading the
        # next page on the next cycle.
        fifo_r_ptr = Signal.like(fifo.r_ptr)
        m.d[self._r_domain] += fifo_r_ptr.eq(fifo.r_ptr)

        read_addr = Cat(self.reader.r_addr[0:self._bits_for_record_size], fifo_r_ptr)
        assert(len(read_addr) == len(r_port.addr))

        m.d.comb += [
            # Memory
            r_port.addr.eq(read_addr),
            self.reader.r_data.eq(r_port.data),
            r_port.en.eq(1),

            # FIFO
            fifo.r_en.eq(self.reader.r_advance & ~self.reader.r_empty),
            # self.reader.r_level.eq(fifo.r_level),
            self.reader.r_level2.eq(Mux(fifo.r_level <= fifo_level_empty, 0, fifo.r_level - fifo_level_empty)),
            self.reader.r_empty.eq(fifo.r_level <= fifo_level_empty),
            self.reader.r_underflow.eq(self.reader.r_advance & self.reader.r_empty),
        ]

        # TODO: Block writes if not ready?

        write_addr = Cat(self.writer.w_addr[0:self._bits_for_record_size], fifo.w_ptr)
        assert(len(write_addr) == len(w_port.addr))

        m.d.comb += [
            # Memory
            w_port.addr.eq(write_addr),
            w_port.data.eq(self.writer.w_data),
            w_port.en.eq(self.writer.w_en),

            # FIFO
            fifo.w_en.eq(self.writer.w_advance & ~self.writer.w_full),
            self.writer.w_level2.eq(Mux(fifo.w_level <= fifo_level_empty, 0, fifo.w_level - fifo_level_empty)),
            self.writer.w_full.eq(fifo.w_level >= fifo_level_full),
            self.writer.w_overflow.eq(self.writer.w_advance & self.writer.w_full),
        ]

        return m
