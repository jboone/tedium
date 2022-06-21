from amaranth import *
from amaranth.hdl.rec import DIR_FANIN, DIR_FANOUT

#######################################################################
# Framer serial interfaces

class RxPhysicalInterface(Record):
    LAYOUT = [
        # Receive Recovered Line Clock Output (RxSCLKn):
        #
        # If receive fractional/signaling interface is enabled -
        # Receive Recovered Line Clock Output (RxSCLKn):
        #
        # These pins output the receoved T1/E1 line clock
        # (1.544MHz in T1 mode and 2.048MHz in E1 mode) for
        # each channel.
        # 
        # NOTE: Receive Fractional/Signaling interface can be
        # enabled by programming to bit 4 - RxFr1544/
        # RxFr2048 bit from register 0xn122 to '1'.
        ('sclk', [
            ('i',  1, DIR_FANOUT),
        ]),

        # Receive Serial Clock Signal (RxSERCLKn):
        #
        # In Base-Rate Mode (1.544MHz/2.048MHz) - RxSERCLKn:
        #
        # These pins are used as the receive serial clock on the
        # system side interface which can be configured as either
        # input or output. The receive serial interface outputs data
        # on RxSERn on the rising edge of RxSERCLKn.
        #
        # When RxSERCLKn is configured as input:
        #
        # These pins will be inputs if the slip buffer on the receive
        # path is enabled. System side equipment must provide a
        # 1.544MHz clock rate to this input pin for T1 mode of oper-
        # ation, and 2.048MHz clock rate in E1 mode.
        ('serclk', [
            ('o',  1, DIR_FANIN),
            ('oe', 1, DIR_FANIN),
        ]),


        # Receive Serial Data Output (RxSERn):
        #
        # DS1/E1 Mode - RxSERn
        # These pins function as the receive serial data output on
        # the system side interface, which are updated on the rising
        # edge of the RxSERCLKn pin. All the framing alignment
        # bits, facility data link bits, CRC bits, and signaling informa-
        # tion will also be extracted to this output pin.
        ('ser', [
            ('i',  1, DIR_FANOUT),
        ]),

        # DS1/E1 base rate mode (1.544MHz/2.048MHz) - RxSYNCn:
        #
        # RxSYNCn pins are used to indicate the single
        # frame boundary within an inbound T1/E1 frame. In both
        # DS1 or E1 mode, the single frame boundary repeats
        # every 125 microseconds (8kHz).
        #
        # In DS1/E1 base rate, RxSYNCn can be configured as
        # either input or output depending on the slip buffer
        # configuration.
        #
        # When RxSYNCn is configured as an input:
        #
        # Users must provide a signal which must pulse "high" for
        # one period of RxSERCLK and repeats every 125us. The
        # receive serial interface will output the first bit of an
        # inbound DS1/E1 frame during the provided RxSYNC pulse.
        #
        # NOTE: It is imperative that the RxSYNC input signal be
        # synchronized with the RxSERCLK input signal.
        ('sync', [
            ('o',  1, DIR_FANIN),
            ('oe', 1, DIR_FANIN),
        ]),

        # RxCRCSYNCn and RxCASYNCn unused.
        ('casync', [
            ('i',  1, DIR_FANOUT),
        ]),

        ('crcsync', [
            ('i',  1, DIR_FANOUT),
        ]),
    ]

    def __init__(self):
        super().__init__(self.LAYOUT, name="rx_phy")
