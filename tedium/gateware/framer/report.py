from amaranth import *

class Report(Record):
    def length_bytes(self):
        record = self.as_value()
        length_bits = len(record)
        assert(length_bits % 8 == 0)
        return length_bits // 8
