from amaranth import *

from .rx_physical import RxPhysicalInterface

class RxRegisteredInterface:
    def __init__(self):
        # Input: serial clock to be sent to framer. Other signals in this
        # interface are launched by the rising edge of this signal.
        self.serclk = Signal()

        # Output: Framer -> FPGA
        # Serial data, captured on SERCLK falling edge.
        self.ser = Signal()

        # Input: FPGA -> framer
        # Frame synchronization pulse, launched on SERCLK rising edge.
        # Framer will synchronize its serial data output and multi-frame
        # synchronization pulses to this synchronization pulse.
        self.sync = Signal()

        # Output: Framer -> FPGA
        # Multi-frame synchronization pulise, captured on SERCLK falling
        # edge.
        self.crcsync = Signal()

        # Output: Framer -> FPGA
        # (Signal only used in E1 mode)
        self.casync = Signal()

class RxRegistered(Elaboratable):
    def __init__(self, phy: RxPhysicalInterface):
        # Input: enable FPGA drivers to framer, after framer
        # interfaces are configured in the appropriate directions.
        self.output_enable = Signal()

        # Physical interface to framer serial RX.
        self.phy = phy

        # Registered interface
        # Output signals are launched toward framer at `serclk` rising edge.
        # Input signals are captured from framer at `serclk` falling edge.
        self.iface = RxRegisteredInterface()

    def elaborate(self, platform) -> Module:
        m = Module()

        phy, iface = self.phy, self.iface

        # Rising edge of SERCLK.
        m.domains.launch = ClockDomain(clk_edge="pos", local=True)
        m.d.comb += ClockSignal("launch").eq(iface.serclk)

        # Falling edge of SERCLK.
        m.domains.capture = ClockDomain(clk_edge="neg", local=True)
        m.d.comb += ClockSignal("capture").eq(iface.serclk)

        # Output enables of FPGA drivers to framer.
        m.d.comb += [
            phy.serclk.oe.eq(self.output_enable),
            phy.sync.oe.eq(self.output_enable),
        ]

        # Capture inputs from framer.
        m.d.capture += [
            iface.ser.eq(phy.ser.i),
            iface.casync.eq(phy.casync.i),
            iface.crcsync.eq(phy.crcsync.i),
        ]

        # Send SERCLK directly to framer.
        m.d.comb += [
            phy.serclk.o.eq(iface.serclk),
        ]

        # Launch outputs to framer.
        m.d.launch += [
            phy.sync.o.eq(iface.sync),
        ]

        return m