from amaranth import *
from amaranth.sim.core import Settle, Simulator

from tedium.gateware.framer.paged_fifo import PagedAsyncFIFO, PagedFIFOReadInterface, PagedFIFOWriteInterface

from tests.hdl import TestCase

class PagedAsyncFIFOTest(TestCase):
    CLOCKS_OVERFLOW = {
        "write": 18.432e6,
        "read": 16.384e6,
    }
    
    CLOCKS_UNDERFLOW = {
        "write": 16.384e6,
        "read": 18.432e6,
    }

    CLOCKS = CLOCKS_UNDERFLOW
    
    def setUp_dut(self):
        m = self.m = Module()

        self.record_size = 7
        self.depth_pages = 8

        dut = self.dut = m.submodules.dut = PagedAsyncFIFO(
            self.record_size, self.depth_pages, "read", "write"
        )

        writer = self.writer = dut.writer
        reader = self.reader = dut.reader

    def test_fifo(self):
        self.setUp_dut()

        dut, writer, reader = self.dut, self.writer, self.reader

        input = [[]]
        output = [[]]

        with self.assertSimulation(self.m, filename="paged_fifo") as sim:
            def write_process():
                count = 0
                for _ in range(100):
                    for n in range(writer.record_size):
                        addr = n
                        yield writer.w_addr.eq(addr)
                        yield writer.w_en.eq(1)
                        value = count
                        yield writer.w_data.eq(value)
                        input[-1].append(value)
                        count += 1

                        yield

                    yield writer.w_en.eq(0)
                    yield writer.w_advance.eq(1)
                    yield
                    yield writer.w_advance.eq(0)

                    input.append([])

            sim.add_sync_process(write_process, domain="write")

            def read_process():
                for _ in range(100):
                    for n in range(reader.record_size):
                        addr = n
                        yield reader.r_addr.eq(addr)

                        yield

                        data = (yield reader.r_data)
                        output[-1].append(data)

                    yield reader.r_advance.eq(1)
                    yield
                    yield reader.r_advance.eq(0)

                    output.append([])

            sim.add_sync_process(read_process, domain="read")

        print(input)
        print(output)

    def test_fifo_overflow_wrapping(self):
        """
        Test that overwriting the FIFO doesn't write the page being read.
        """
        self.setUp_dut()

        dut, writer, reader = self.dut, self.writer, self.reader

        with self.assertSimulation(self.m, filename="paged_fifo_overflow_wrapping") as sim:
            def process():
                w_datas_in = []
                # Fill up FIFO, and over-write once.
                for page in range(self.depth_pages + 1):
                    for addr in range(self.record_size):
                        w_data = page * self.record_size + addr
                        w_datas_in.append(w_data)
                        yield writer.w_addr.eq(addr)
                        yield writer.w_data.eq(w_data)
                        yield writer.w_en.eq(1)
                        yield
                
                    yield writer.w_en.eq(0)
                    yield writer.w_advance.eq(1)
                    yield
                    yield writer.w_advance.eq(0)

                r_datas_out = []
                for addr in range(self.record_size):
                    yield reader.r_addr.eq(addr)
                    yield

                    r_data = (yield reader.r_data)
                    r_datas_out.append(r_data)

                self.assertEqual(r_datas_out, w_datas_in[:self.record_size])
                yield

            sim.add_sync_process(process, domain="write")

    def test_fifo_underflow_wrapping(self):
        """
        Test that overreading the FIFO doesn't read past the last page that was written.
        """
        self.setUp_dut()

        dut, writer, reader = self.dut, self.writer, self.reader

        self.write_done = False
        self.w_datas_in = []
        self.r_datas_out = []

        with self.assertSimulation(self.m, filename="paged_fifo_underflow_wrapping") as sim:
            def process_write():
                # Fill up FIFO.
                for page in range(self.depth_pages):
                    for addr in range(self.record_size):
                        w_data = page * self.record_size + addr
                        self.w_datas_in.append(w_data)
                        yield writer.w_addr.eq(addr)
                        yield writer.w_data.eq(w_data)
                        yield writer.w_en.eq(1)
                        yield
                
                    yield writer.w_en.eq(0)
                    yield writer.w_advance.eq(1)
                    yield
                    yield writer.w_advance.eq(0)

                self.write_done = True

            sim.add_sync_process(process_write, domain="write")

            def process_read():
                while self.write_done == False:
                    yield

                # Now, overread the full FIFO. The overread page
                # (the page before the one the write pointer is at)
                # should read the same as before.
                for page in range(self.depth_pages):
                    for addr in range(self.record_size):
                        yield reader.r_addr.eq(addr)
                        yield

                        yield Settle()
                        r_data = (yield reader.r_data)
                        self.r_datas_out.append(r_data)

                    yield reader.r_advance.eq(1)
                    yield
                    yield reader.r_advance.eq(0)

                # We should not be reading the page that the write pointer
                # is still pointing at. So expect the last 
                # self.assertEqual(r_datas_out, w_datas_in[:self.record_size])
                r_datas_expected = self.w_datas_in[:-self.record_size]
                r_datas_expected.extend(self.w_datas_in[-2*self.record_size:-1*self.record_size])

                self.assertEqual(self.r_datas_out, r_datas_expected)

            sim.add_sync_process(process_read, domain="read")

