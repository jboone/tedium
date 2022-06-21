from amaranth import *

from .tx_physical import TxPhysicalInterface

class TxRegisteredInterface:
    """
    Serial data and frame-synchronization to the framer TX physical interface.
    Signals are captured on the rising edge of SERCLK (provided )
    """
    def __init__(self):
        # Input: serial clock to be sent to framer. Other signals in this
        # interface are launched by the rising edge of this signal.
        self.serclk = Signal()

        # Input: FPGA -> framer
        # Serial data, launched on `serclk` rising edge.
        self.ser = Signal()

        # Input: FPGA -> framer
        # Frame synchronization pulse, launched on `serclk` rising edge.
        self.sync = Signal()

        # Input: FPGA -> framer
        # Multi-frame synchronization pulse, launched on `serclk` rising edge.
        self.msync = Signal()

class TxRegistered(Elaboratable):
    """
    Wrapper around physical/electrical interface to framer transmit
    serial interface. Launches signals on rising edge of `serclk`.
    """
    def __init__(self, phy: TxPhysicalInterface):
        # Input: enable FPGA drivers to framer, after framer
        # interfaces are configured in the appropriate directions.
        self.output_enable = Signal()

        # Registered interface
        # Output signals are launched toward framer at `serclk` rising edge.
        self.iface = TxRegisteredInterface()

        # Physical interface to framer serial TX.
        self.phy = phy

    def elaborate(self, platform) -> Module:
        m = Module()

        phy, iface = self.phy, self.iface

        m.domains.launch = ClockDomain(clk_edge="pos", local=True)
        m.d.comb += ClockSignal("launch").eq(iface.serclk)

        # Output enables of FPGA drivers to framer.
        m.d.comb += [
            phy.serclk.oe.eq(self.output_enable),
            # phy.ser.oe.eq(self.output_enable),
            phy.sync.oe.eq(self.output_enable),
            phy.msync.oe.eq(self.output_enable),
        ]

        # Send SERCLK directly to framer.
        m.d.comb += [
            phy.serclk.o.eq(iface.serclk),
        ]

        # Launch outputs to framer.
        m.d.launch += [
            phy.ser.o.eq(iface.ser),
            phy.sync.o.eq(iface.sync),
            phy.msync.o.eq(iface.msync),
        ]

        return m
