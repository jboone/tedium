from amaranth import *

class TimeslotInterface:
    def __init__(self):
        # Timeslot data bytes, aligned with system timing.
        self.data = Signal(8)

        # Frame F-bit, changes at start of new system timing frame.
        self.f = Signal()

        # Multi-frame indicator, changes at start of new system timing frame.
        self.mf = Signal()
