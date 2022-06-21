from amaranth import *
from amaranth.hdl.rec import DIR_FANIN

class TxPhysicalInterface(Record):
    LAYOUT = [
        # Serial clock for serial data to the framer, can also be
        # used as a clock source for the transmitter.
        ('serclk', [
            ('o',  1, DIR_FANIN),
            ('oe', 1, DIR_FANIN),
        ]),

        # Serial data to framer for transmission to remote system.
        ('ser', [
            ('o',  1, DIR_FANIN),
            # ('oe', 1, DIR_FANIN),
        ]),

        # Frame synchronization pulse during the F bit time of the
        # serial data.
        ('sync', [
            ('o',  1, DIR_FANIN),
            ('oe', 1, DIR_FANIN),
        ]),

        # Multi-frame synchonization pulse during the F bit time of
        # the first F bit in a multi-frame structure (e.g. SF, ESF).
        ('msync', [
            ('o',  1, DIR_FANIN),
            ('oe', 1, DIR_FANIN),
        ]),
    ]

    def __init__(self):
        super().__init__(self.LAYOUT, name="tx_phy")
