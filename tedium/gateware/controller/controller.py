from amaranth import *
from amaranth.utils import log2_int
from amaranth_soc import wishbone
from amaranth_soc.memory import MemoryMap
from amaranth_stdio.serial import AsyncSerial

from lambdasoc.cpu.minerva import MinervaCPU
from lambdasoc.periph.base import Peripheral
from lambdasoc.periph.intc import GenericInterruptController
from lambdasoc.periph.serial import AsyncSerialPeripheral
from lambdasoc.periph.sram import SRAMPeripheral
from lambdasoc.periph.timer import TimerPeripheral
from lambdasoc.soc.cpu import CPUSoC

from luna.gateware.usb.usb2.interfaces.eptri import InFIFOInterface

from tedium.gateware.framer.microprocessor import MicroprocessorInterface
from tedium.gateware.usb.descriptors_vendor import Descriptors

class TestPointPeripheral(Peripheral, Elaboratable):
    def __init__(self, pins):
        super().__init__()

        self._pins = pins

        bank = self.csr_bank()
        self._output = bank.csr(len(pins), "rw")

        self._bridge = self.bridge(data_width=32, granularity=8, alignment=2)
        self.bus = self._bridge.bus

    # @property
    # def constant_map(self):
    #     return ConstantMap(
    #         SIZE = self.size,
    #     )

    def elaborate(self, platform) -> Module:
        m = Module()
        m.submodules.bridge = self._bridge

        with m.If(self._output.w_stb):
            m.d.sync += [
                self._output.r_data.eq(self._output.w_data),
                self._pins.tp9.o .eq(self._output.w_data[0]),
                self._pins.tp7.o .eq(self._output.w_data[1]),
                self._pins.tp10.o.eq(self._output.w_data[2]),
            ]

        return m

class FramerControlPeripheral(Peripheral, Elaboratable):
    def __init__(self, pins):
        super().__init__()

        self._pins = pins

        bank = self.csr_bank()
        self._reset     = bank.csr(1, "rw")
        self._if_enable = bank.csr(1, "rw")

        self._bridge = self.bridge(data_width=32, granularity=8, alignment=2)
        self.bus = self._bridge.bus

    def elaborate(self, platform) -> Module:
        m = Module()
        m.submodules.bridge = self._bridge

        with m.If(self._reset.w_stb):
            m.d.sync += [
                self._reset.r_data.eq(self._reset.w_data),
                self._pins.reset.eq(self._reset.w_data),
            ]

        with m.If(self._if_enable.w_stb):
            m.d.sync += [
                self._if_enable.r_data.eq(self._if_enable.w_data),
                self._pins.output_enable.eq(self._if_enable.w_data),
            ]

        return m

class FramerRegistersPeripheral(Peripheral, Elaboratable):
    def __init__(self):
        super().__init__()

        self.name = "framer"

        size = 0x8000 * 4
        data_width = 8
        granularity = 8
        mem_depth = (size * granularity) // data_width
        
        self.bus = wishbone.Interface(addr_width=log2_int(mem_depth), data_width=data_width, granularity=granularity)

        map = MemoryMap(addr_width=log2_int(size), data_width=granularity, name=self.name)
        map.add_resource("framer", name=self.name, size=size)
        self.bus.memory_map = map

        self.size = size
        self.granularity = granularity

    # @property
    # def constant_map(self):
    #     return ConstantMap(
    #         SIZE = self.size,
    #     )

    def elaborate(self, platform) -> Module:
        m = Module()

        wb = self.bus
        up_bus = platform.request('microprocessor_bus')
        up = m.submodules.iface = MicroprocessorInterface(bus=up_bus)

        selected = wb.cyc & wb.stb
        selected_q1 = Signal()
        m.d.sync += selected_q1.eq(selected)

        selected_stb = Signal()
        m.d.sync += selected_stb.eq(selected & ~selected_q1)

        # assert(len(wb.adr) == 15)

        m.d.comb += [
            up.start.eq(selected_stb),
            wb.ack.eq(up.done),
            up.address.eq(wb.adr),
            up.data_wr.eq(wb.dat_w),
            wb.dat_r.eq(up.data_rd),
            up.write.eq(wb.we),
        ]

        return m

class SoC(CPUSoC, Elaboratable):
    BOOTROM_ADDRESS     = 0x0000_0000
    SCRATCHPAD_ADDRESS  = 0x0000_8000

    MAINRAM_ADDRESS     = 0x4000_0000

    UART_ADDRESS        = 0x8000_0000
    TIMER_ADDRESS       = 0x8000_1000
    TP_ADDRESS          = 0x8000_2000
    FRAMER_CTRL_ADDRESS = 0x8000_3000

    # USB_CORE_ADDRESS    = 0x8005_0000
    # USB_SETUP_ADDRESS   = 0x8006_0000
    # USB_IN_EP0_ADDRESS  = 0x8007_0000
    # USB_OUT_EP0_ADDRESS = 0x8008_0000
    USB_IN_INT_ADDRESS  = 0x8009_0000
    # USB_OUT_INT_ADDRESS = 0x800a_0000

    FRAMER_REG_ADDRESS  = 0x8010_0000

    def __init__(self):
        self.uart_pins = Record([
            ('rx', [('i', 1)]),
            ('tx', [('o', 1)]),
        ])

        self.tp_pins = Record([
            ('tp9', [('o', 1)]),
            ('tp7', [('o', 1)]),
            ('tp10', [('o', 1)]),
        ])

        self.framer_control_pins = Record([
            ('reset',         [('o', 1)]),
            ('output_enable', [('o', 1)]),
        ])

        sync_clk_freq = 60.0e6
        baudrate      = 115200

        mainram_size = 0x4000

        uart_core = AsyncSerial(
            data_bits = 8,
            divisor   = int(sync_clk_freq // baudrate),
            pins      = self.uart_pins,
        )

        self.sync_clk_freq = int(sync_clk_freq)

        self.cpu = MinervaCPU(
            reset_address = self.BOOTROM_ADDRESS,
            with_icache   = True,
            icache_nlines = 16,
            icache_nwords = 4,
            icache_nways  = 1,
            icache_base   = self.MAINRAM_ADDRESS,
            icache_limit  = self.MAINRAM_ADDRESS + mainram_size,
            with_dcache   = True,
            dcache_nlines = 16,
            dcache_nwords = 4,
            dcache_nways  = 1,
            dcache_base   = self.MAINRAM_ADDRESS,
            dcache_limit  = self.MAINRAM_ADDRESS + mainram_size,
            with_muldiv   = True,
        )

        bootrom_size    = 0x8000
        scratchpad_size = 0x1000

        timer_width     = 32

        self._arbiter = wishbone.Arbiter(addr_width=30, data_width=32, granularity=8,
                                         features={"cti", "bte", "err"})
        self._arbiter.add(self.cpu.ibus)
        self._arbiter.add(self.cpu.dbus)

        self.bus_decoder = wishbone.Decoder(addr_width=30, data_width=32, granularity=8,
                                         features={"cti", "bte", "err"})

        self.intc = GenericInterruptController(width=len(self.cpu.ip))

        self._submodules = []
        self._irqs = {}
        self._next_irq_index = 0

        self.bootrom = SRAMPeripheral(size=bootrom_size, writable=False)
        self.bus_decoder.add(self.bootrom.bus, addr=self.BOOTROM_ADDRESS)

        self.scratchpad = SRAMPeripheral(size=scratchpad_size)
        self.bus_decoder.add(self.scratchpad.bus, addr=self.SCRATCHPAD_ADDRESS)

        self.uart = AsyncSerialPeripheral(core=uart_core)
        self.add_peripheral(self.uart, addr=self.UART_ADDRESS)

        self.timer = TimerPeripheral(width=timer_width)
        self.add_peripheral(self.timer, addr=self.TIMER_ADDRESS)

        self.framer_ctrl = FramerControlPeripheral(pins=self.framer_control_pins)
        self.add_peripheral(self.framer_ctrl, addr=self.FRAMER_CTRL_ADDRESS)

        self._sram = SRAMPeripheral(size=mainram_size)
        self.bus_decoder.add(self._sram.bus, addr=self.MAINRAM_ADDRESS)

        self._sdram  = None
        self._ethmac = None

        self.tp = TestPointPeripheral(pins=self.tp_pins)
        self.add_peripheral(self.tp, addr=self.TP_ADDRESS)

        self.framer = FramerRegistersPeripheral()
        self.add_peripheral(self.framer, addr=self.FRAMER_REG_ADDRESS, sparse=True)

        # self.usb_device_controller = USBDeviceController()
        # self.add_peripheral(self.usb_device_controller, addr=self.USB_CORE_ADDRESS)

        # self.usb_setup = SetupFIFOInterface()
        # self.add_peripheral(self.usb_setup, as_submodule=False, addr=self.USB_SETUP_ADDRESS)

        # self.usb_in_ep_0 = InFIFOInterface(endpoint_number=0, max_packet_size=64)
        # self.add_peripheral(self.usb_in_ep_0, as_submodule=False, addr=self.USB_IN_EP0_ADDRESS)

        self.usb_in_ep_interrupt = InFIFOInterface(endpoint_number=2, max_packet_size=Descriptors.INTERRUPT_BYTES_MAX)
        self.add_peripheral(self.usb_in_ep_interrupt, as_submodule=False, addr=self.USB_IN_INT_ADDRESS)

        # self.usb_out_ep = OutFIFOInterface()
        # self.add_peripheral(self.usb_out_ep, as_submodule=False, addr=self.USB_OUT_ADDRESS)

    @property
    def memory_map(self):
        return self.bus_decoder.bus.memory_map

    @property
    def constants(self):
        return super().constants.union(
            # SOC    = ConstantMap(
            #     MEMTEST_ADDR_SIZE = 8192,
            #     MEMTEST_DATA_SIZE = 8192,
            # ),
            # TP = self.tp.constant_map,
            # FRAMER = self.framer.constant_map,
        )

    @property
    def mainram(self):
        assert not (self._sdram and self.sram)
        return self._sdram or self._sram

    @property
    def sdram(self):
        return self._sdram

    @property
    def sram(self):
        return self._sram

    @property
    def ethmac(self):
        return self._ethmac

    def add_peripheral(self, p, *, as_submodule=True, **kwargs):
        """ Adds a peripheral to the SoC.

        For now, this is identical to adding a peripheral to the SoC's wishbone bus.
        For convenience, returns the peripheral provided.
        """

        # Add the peripheral to our bus...
        interface = getattr(p, 'bus')
        self.bus_decoder.add(interface, **kwargs)

        # ... add its IRQs to the IRQ controller...
        try:
            irq_line = getattr(p, 'irq')
            self.intc.add_irq(irq_line, self._next_irq_index)

            self._irqs[self._next_irq_index] = p
            self._next_irq_index += 1
        except (AttributeError, NotImplementedError):

            # If the object has no associated IRQs, continue anyway.
            # This allows us to add devices with only Wishbone interfaces to our SoC.
            pass

        # ... and keep track of it for later.
        if as_submodule:
            self._submodules.append(p)

        return p

    def elaborate(self, platform) -> Module:
        m = Module()

        m.submodules.cpu        = self.cpu
        m.submodules.arbiter    = self._arbiter
        m.submodules.decoder    = self.bus_decoder
        m.submodules.intc       = self.intc
        m.submodules.bootrom    = self.bootrom
        m.submodules.scratchpad = self.scratchpad

        if self.sdram is not None:
            m.submodules.sdram = self.sdram
        if self.sram is not None:
            m.submodules.sram = self.sram
        if self.ethmac is not None:
            m.submodules.ethmac = self.ethmac

        for peripherals in self._submodules:
            m.submodules += peripherals

        m.d.comb += [
            self._arbiter.bus.connect(self.bus_decoder.bus),
            self.cpu.ip.eq(self.intc.ip),
        ]

        return m
