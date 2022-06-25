#!/usr/bin/env python3

#
# This file is part of Tedium.
#
# Copyright (C) 2020 Jared Boone <jared@sharebrained.com>
# SPDX-License-Identifier: BSD-3-Clause

import os
import subprocess

from amaranth import *

from amaranth.build import *
from amaranth.vendor.lattice_ecp5 import *

from amaranth_boards.resources import UARTResource

from luna.gateware.platform.core import LUNAPlatform

__all__ = ["TediumX8Platform"]

def ULPIResource(*args, data, clk, dir, nxt, stp, rst=None,
			clk_dir='i', rst_invert=False, attrs=None, conn=None):
	# NOTE: Borrowed from amaranth-boards, but with "clk" subsignal Clock(60e6) qualifier to enable timing closure for the USB clock.

	assert clk_dir in ('i', 'o',)

	io = []
	io.append(Subsignal("data", Pins(data, dir="io", conn=conn, assert_width=8)))
	io.append(Subsignal("clk", Pins(clk, dir=clk_dir, conn=conn, assert_width=1), Clock(60e6)))
	io.append(Subsignal("dir", Pins(dir, dir="i", conn=conn, assert_width=1)))
	io.append(Subsignal("nxt", Pins(nxt, dir="i", conn=conn, assert_width=1)))
	io.append(Subsignal("stp", Pins(stp, dir="o", conn=conn, assert_width=1)))
	if rst is not None:
		io.append(Subsignal("rst", Pins(rst, dir="o", invert=rst_invert,
			conn=conn, assert_width=1)))
	if attrs is not None:
		io.append(attrs)
	return Resource.family(*args, default_name="usb", ios=io)

def MicroprocessorBusResource(name, addr_sites, data_sites, pclk_site, cs_site, ale_as_site, rd_ds_we_site, wr_rw_site, rdy_dtack_site, int_site, req_sites, ack_sites, ptype0_site, ptype2_site, reset_site):
	""" Generates a set of resource for the XRT86VX38 TDM LIU/framer microprocessor interface. """

	# PTYPE1 is permanently grounded.

	return Resource(name, 0,
		Subsignal("addr",   Pins (addr_sites,     dir="o" )),
		Subsignal("data",   Pins (data_sites,     dir="io")),
		Subsignal("pclk",   Pins (pclk_site,      dir="o" )),
		Subsignal("cs",     PinsN(cs_site,        dir="o" ), Attrs(PULLMODE="UP")),
		Subsignal("ale",    Pins (ale_as_site,    dir="o" )),
		Subsignal("rd",     PinsN(rd_ds_we_site,  dir="o" ), Attrs(PULLMODE="UP")),
		Subsignal("wr",     PinsN(wr_rw_site,     dir="o" ), Attrs(PULLMODE="UP")),
		Subsignal("rdy",    PinsN(rdy_dtack_site, dir="i" )),
		Subsignal("int",    PinsN(int_site,       dir="i" )),
		Subsignal("req",    PinsN(req_sites,      dir="i" )),
		Subsignal("ack",    PinsN(ack_sites,      dir="o" )),
		Subsignal("ptype0", Pins (ptype0_site,    dir="o" )),
		Subsignal("ptype2", Pins (ptype2_site,    dir="o" )),
		Subsignal("reset",  PinsN(reset_site,     dir="o" )),
		Attrs(IO_TYPE="LVCMOS33", SLEWRATE="SLOW", DRIVE="4"),
	)

def FramerRXResource(ordinal, sync_neg_site, crcsync_site, casync_site, serclk_lineclk_site, ser_pos_site, sclk_site, mode='test'):
	if mode == 'test':
		return Resource("rx", ordinal,
			Subsignal("sync",    Pins(sync_neg_site,       dir="io")),
			Subsignal("crcsync", Pins(crcsync_site,        dir="i" )),
			Subsignal("casync",  Pins(casync_site,         dir="i" )),
			Subsignal("serclk",  Pins(serclk_lineclk_site, dir="io")),
			Subsignal("ser",     Pins(ser_pos_site,        dir="i" )),
			Subsignal("sclk",    Pins(sclk_site,           dir="i" )),
			Attrs(IO_TYPE="LVCMOS33", SLEWRATE="SLOW", DRIVE="4"),
		)
	else:
		raise RuntimeError('mode not specified')

def FramerTXResource(ordinal, ser_pos_site, serclk_lineclk_site, sync_neg_site, msync_inclk_site, mode='test'):
	if mode == 'test':
		return Resource("tx", ordinal,
			Subsignal("ser",    Pins(ser_pos_site,        dir="o" )),
			Subsignal("serclk", Pins(serclk_lineclk_site, dir="io")),
			Subsignal("sync",   Pins(sync_neg_site,       dir="io")),
			Subsignal("msync",  Pins(msync_inclk_site,    dir="io")),
			Attrs(IO_TYPE="LVCMOS33", SLEWRATE="SLOW", DRIVE="4"),
		)
	else:
		raise RuntimeError('mode not specified')

class TediumECP5DomainGenerator(Elaboratable):

	def __init__(self, *, clock_frequencies=None, clock_signal_name=None):
		pass

	def elaborate(self, platform):
		m = Module()

		# Create our clock domains.
		m.domains.fast   = ClockDomain()
		m.domains.sync   = ClockDomain()
		m.domains.usb    = ClockDomain()

		# Grab our clock and global reset signals.
		clk_16m384 = platform.request(platform.default_clk)
		reset = Const(0)

		# Generate the clocks we need for our PLL.
		feedback = Signal()
		locked   = Signal()

		pll_config = 'slow'

		if pll_config == 'fast':
			m.submodules.pll = Instance("EHXPLLL",
				i_CLKI             = clk_16m384,

				o_CLKOP            = ClockSignal("fast"),
				o_CLKOS            = ClockSignal("sync"),

				# Status.
				o_LOCK             = locked,

				i_CLKFB            = ClockSignal("fast"),

				# Control signals.
				i_RST              = reset,
				i_STDBY            = 0,
				# i_CLKINTFB         = 0,
				i_PHASESEL0        = 0,
				i_PHASESEL1        = 0,
				i_PHASEDIR         = 1,
				i_PHASESTEP        = 1,
				i_PHASELOADREG     = 1,
				i_PLLWAKESYNC      = 0,
				i_ENCLKOP          = 0,
				i_ENCLKOS          = 0,
				i_ENCLKOS2         = 0,
				i_ENCLKOS3         = 0,

				p_PLLRST_ENA       = "DISABLED",
				p_INTFB_WAKE       = "DISABLED",
				p_STDBY_ENABLE     = "DISABLED",
				p_DPHASE_SOURCE    = "DISABLED",

				p_CLKI_DIV         = 1,

				p_CLKOP_ENABLE     = "ENABLED",
				p_CLKOP_DIV        = 4,
				p_CLKOP_CPHASE     = 1,
				p_CLKOP_FPHASE     = 0,
				# p_CLKOP_TRIM_DELAY = 0,
				# p_CLKOP_TRIM_POL   = "FALLING",

				p_CLKOS_ENABLE     = "ENABLED",
				p_CLKOS_DIV        = 8,
				p_CLKOS_CPHASE     = 1,
				p_CLKOS_FPHASE     = 0,
				# p_CLKOS_TRIM_DELAY = 0,
				# p_CLKOS_TRIM_POL   = "FALLING",

				p_FEEDBK_PATH      = "CLKOP",
				p_CLKFB_DIV        = 12,

				p_CLKOS3_FPHASE    = 0,
				p_CLKOS3_CPHASE    = 0,
				p_CLKOS2_FPHASE    = 0,
				p_CLKOS2_CPHASE    = 0,
				p_PLL_LOCK_MODE    = 0,
				p_OUTDIVIDER_MUXD  = "DIVD",
				p_CLKOS3_ENABLE    = "DISABLED",
				p_OUTDIVIDER_MUXC  = "DIVC",
				p_CLKOS2_ENABLE    = "DISABLED",
				p_OUTDIVIDER_MUXB  = "DIVB",
				p_OUTDIVIDER_MUXA  = "DIVA",
				p_CLKOS3_DIV       = 1,
				p_CLKOS2_DIV       = 1,

				# Synthesis attributes.
				a_FREQUENCY_PIN_CLKI="16.384000",
				a_FREQUENCY_PIN_CLKOP="196.608000",
				a_FREQUENCY_PIN_CLKOS="98.304000",

				a_ICP_CURRENT="12",
				a_LPF_RESISTOR="8",
				a_MFG_ENABLE_FILTEROPAMP="1",
				a_MFG_GMCREF_SEL="2",
			)
		else:
			m.submodules.pll = Instance("EHXPLLL",
				i_CLKI             = clk_16m384,	# TODO: Explicitly put through a buffer?

				o_CLKOP            = ClockSignal("fast"),
				o_CLKOS            = ClockSignal("sync"),

				# Status.
				o_LOCK             = locked,

				i_CLKFB            = feedback,
				o_CLKINTFB         = feedback,

				# Control signals.
				i_RST              = reset,
				i_STDBY            = 0,
				i_PHASESEL0        = 0,
				i_PHASESEL1        = 0,
				i_PHASEDIR         = 0,
				i_PHASESTEP        = 0,
				i_PHASELOADREG     = 0,
				i_PLLWAKESYNC      = 0,
				i_ENCLKOP          = 0,
				i_ENCLKOS          = 0,
				i_ENCLKOS2         = 0,
				i_ENCLKOS3         = 0,

				p_PLLRST_ENA       = "DISABLED",
				p_INTFB_WAKE       = "DISABLED",
				p_STDBY_ENABLE     = "DISABLED",
				p_DPHASE_SOURCE    = "DISABLED",

				p_CLKI_DIV         = 1,

				p_CLKOP_ENABLE     = "ENABLED",
				p_CLKOP_DIV        = 7,
				p_CLKOP_FPHASE     = 0,
				p_CLKOP_CPHASE     = 6,
				p_CLKOP_TRIM_DELAY = 0,
				p_CLKOP_TRIM_POL   = "FALLING",

				p_CLKOS_ENABLE     = "ENABLED",
				p_CLKOS_DIV        = 14,
				p_CLKOS_FPHASE     = 0,
				p_CLKOS_CPHASE     = 13,
				p_CLKOS_TRIM_DELAY = 0,
				p_CLKOS_TRIM_POL   = "FALLING",

				p_FEEDBK_PATH      = "INT_OP",
				p_CLKFB_DIV        = 6,

				p_CLKOS3_FPHASE    = 0,
				p_CLKOS3_CPHASE    = 0,
				p_CLKOS2_FPHASE    = 0,
				p_CLKOS2_CPHASE    = 0,
				p_PLL_LOCK_MODE    = 0,
				p_OUTDIVIDER_MUXD  = "DIVD",
				p_CLKOS3_ENABLE    = "DISABLED",
				p_OUTDIVIDER_MUXC  = "DIVC",
				p_CLKOS2_ENABLE    = "DISABLED",
				p_OUTDIVIDER_MUXB  = "DIVB",
				p_OUTDIVIDER_MUXA  = "DIVA",
				p_CLKOS3_DIV       = 1,
				p_CLKOS2_DIV       = 1,

				# # Synthesis attributes.
				a_FREQUENCY_PIN_CLKI="16.384000",
				a_FREQUENCY_PIN_CLKOP="98.304000",
				a_FREQUENCY_PIN_CLKOS="49.152000",

				a_ICP_CURRENT="6",
				a_LPF_RESISTOR="16",
				# a_MFG_ENABLE_FILTEROPAMP="1",
				# a_MFG_GMCREF_SEL="2",
			)

		m.d.comb += [

			ResetSignal("sync"  ).eq(~locked),
			ResetSignal("fast"  ).eq(~locked),
			# ResetSignal("framer").eq(0),
		]

		return m

	# def generate_usb_clock(self, m, platform):
		# return self._clock_options[self.clock_frequencies['usb']]

	# def generate_sync_clock(self, m, platform):
	# 	return self._clock_options[self.clock_frequencies['sync']]

	# def generate_fast_clock(self, m, platform):
	# 	return self._clock_options[self.clock_frequencies['fast']]

	# def generate_framer_clock(self, m, platform):
	# 	return self._clock_options[self.clock_frequencies['framer']]

	# def stretch_sync_strobe_to_usb(self, m, strobe, output=None, allow_delay=False):
	# 	to_cycles = self.clock_frequencies['sync'] // self.clock_frequencies['usb']
	# 	return stretch_strobe_signal(m, strobe, output=output, to_cycles=to_cycles, allow_delay=allow_delay)

class TediumX8Platform(LatticeECP5Platform, LUNAPlatform):
	name        = "Tedium X8"

	device      = "LFE5U-85F"
	package     = "BG381"
	speed       = "8"

	default_clk = "clk_16m384"

	clock_domain_generator = TediumECP5DomainGenerator

	default_usb_connection = "ulpi"

	DEFAULT_CLOCK_FREQUENCIES_MHZ = {
		"fast"  : 196.608,
		"sync"  :  98.304,
		"usb"   :  60.000,
	}

	resources = [
		Resource("clk_16m384", 0, Pins( "U16", dir="i"), Clock(16.384e6), Attrs(IO_TYPE="LVCMOS33")),
		Resource("program",    0, PinsN("P1",  dir="o"),                  Attrs(IO_TYPE="LVCMOS33")),

		Resource("mclkin",     0, Pins( "K20", dir="o"),                  Attrs(IO_TYPE="LVCMOS33")),
		Resource("t1oscclk",   0, Pins( "P3",  dir="i"),                  Attrs(IO_TYPE="LVCMOS33")),
		Resource("e1oscclk",   0, Pins( "P4",  dir="i"),                  Attrs(IO_TYPE="LVCMOS33")),
		Resource("extosc8k",   0, Pins( "M1",  dir="o"),                  Attrs(IO_TYPE="LVCMOS33")),
		Resource("txon",       0, Pins( "L2",  dir="o"),                  Attrs(IO_TYPE="LVCMOS33")),
		Resource("rxtsel",     0, Pins( "J19", dir="o"),                  Attrs(IO_TYPE="LVCMOS33")),

		Resource("tp7",        0, Pins( "N1",  dir="o"),                  Attrs(IO_TYPE="LVCMOS33")),
		Resource("tp9",        0, Pins( "P2",  dir="o"),                  Attrs(IO_TYPE="LVCMOS33")),
		Resource("tp10",       0, Pins( "C4",  dir="o"),                  Attrs(IO_TYPE="LVCMOS33")),

		# FTDI-attached UART

		UARTResource(0,
			rx="H4",
			tx="V1",
			rts="U1",
			cts="T1",
			role="dce",
			attrs=Attrs(IO_TYPE="LVCMOS33", PULLMODE="UP"),
		),

		# Span 0

		FramerTXResource(0,
			ser_pos_site="L18",
			serclk_lineclk_site="K18",
			sync_neg_site="K19",
			msync_inclk_site="J20",
		),

		FramerRXResource(0,
			sync_neg_site="F19",
			crcsync_site="G20",
			casync_site="F20",
			serclk_lineclk_site="G19",
			ser_pos_site="H20",
			sclk_site="E19",
		),

		# Span 1

		FramerTXResource(1,
			ser_pos_site="H18",
			serclk_lineclk_site="A19",
			sync_neg_site="B20",
			msync_inclk_site="C20",
		),

		FramerRXResource(1,
			sync_neg_site="D20",
			crcsync_site="L20",
			casync_site="K17",
			serclk_lineclk_site="E20",
			ser_pos_site="D19",
			sclk_site="J18",
		),

		# Span 2

		FramerTXResource(2,
			ser_pos_site="D16",
			serclk_lineclk_site="A17",
			sync_neg_site="B18",
			msync_inclk_site="A16",
		),

		FramerRXResource(2,
			sync_neg_site="B19",
			crcsync_site="E18",
			casync_site="E17",
			serclk_lineclk_site="D18",
			ser_pos_site="A18",
			sclk_site="B17",
		),

		# Span 3

		FramerTXResource(3,
			ser_pos_site="A13",
			serclk_lineclk_site="C13",
			sync_neg_site="A14",
			msync_inclk_site="C17",
		),

		FramerRXResource(3,
			sync_neg_site="B15",
			crcsync_site="J17",
			casync_site="C16",
			serclk_lineclk_site="A15",
			ser_pos_site="B16",
			sclk_site="G18",
		),

		# Span 4

		FramerTXResource(4,
			ser_pos_site="D3",
			serclk_lineclk_site="A2",
			sync_neg_site="B1",
			msync_inclk_site="E6",
		),

		FramerRXResource(4,
			sync_neg_site="C7",
			crcsync_site="B4",
			casync_site="B3",
			serclk_lineclk_site="E7",
			ser_pos_site="B5",
			sclk_site="A3",
		),

		# Span 5

		FramerTXResource(5,
			ser_pos_site="H3",
			serclk_lineclk_site="G3",
			sync_neg_site="E4",
			msync_inclk_site="D2",
		),

		FramerRXResource(5,
			sync_neg_site="E3",
			crcsync_site="B2",
			casync_site="C2",
			serclk_lineclk_site="C1",
			ser_pos_site="E2",
			sclk_site="D1",
		),

		# Span 6

		FramerTXResource(6,
			ser_pos_site="G1",
			serclk_lineclk_site="H1",
			sync_neg_site="L3",
			msync_inclk_site="G2",
		),

		FramerRXResource(6,
			sync_neg_site="F1",
			crcsync_site="F2",
			casync_site="J3",
			serclk_lineclk_site="C3",
			ser_pos_site="E1",
			sclk_site="J4",
		),

		# Span 7

		FramerTXResource(7,
			ser_pos_site="K1",
			serclk_lineclk_site="N3",
			sync_neg_site="K2",
			msync_inclk_site="N4",
		),

		FramerRXResource(7,
			sync_neg_site="H2",
			crcsync_site="M4",
			casync_site="F3",
			serclk_lineclk_site="J1",
			ser_pos_site="L4",
			sclk_site="K3",
		),

		# USB ULPI

		ULPIResource("ulpi", 0,
			data="T19 T20 R18 R20 P19 P20 N19 N20",
			clk="L19",
			clk_dir='i',
			dir="M19",
			nxt="M20",
			stp="U20",
			rst="U19",
			rst_invert=True,
			attrs=Attrs(IO_TYPE="LVCMOS33", SLEWRATE="FAST"),
		),

		MicroprocessorBusResource("microprocessor_bus",
			addr_sites="E9 C9 B9 C5 D9 A9 E10 B10 C10 C11 D13 A11 A12 B13 H17",
			data_sites="E8 D10 E11 B11 C12 E14 F18 C15",
			pclk_site="A7",
			cs_site="C14",
			ale_as_site="A10",
			rd_ds_we_site="B8",
			wr_rw_site="E15",
			rdy_dtack_site="A8",
			int_site="B12",
			req_sites="B6 A4",
			ack_sites="A5 A6",
			ptype0_site="D7",
			ptype2_site="D12",
			reset_site="L1",
		)
	]

	connectors = [
	]

	@property
	def file_templates(self):
		return {
			**super().file_templates,
			"{{name}}-openocd.cfg": r"""
			adapter driver ftdi
			ftdi vid_pid 0x0403 0x6010
			ftdi channel 0
			ftdi layout_init 0xfff8 0xfffb
			ftdi tdo_sample_edge falling
			reset_config none
			adapter speed 25000
			transport select jtag
			jtag newtap ecp5 tap -irlen 8 -expected-id 0x41113043
			"""
		}
		
	def toolchain_program(self, products, name):
		openocd = os.environ.get("OPENOCD", "openocd")
		with products.extract("{}-openocd.cfg".format(name), "{}.svf".format(name)) \
				as (config_filename, vector_filename):
			subprocess.check_call([openocd,
				"-f", config_filename,
				"-c", "transport select jtag; init; svf -quiet {}; exit".format(vector_filename)
			])

# class TediumX8Platform(_TediumX8Platform, LUNAPlatform):
# 	def __init__(self, *args, **kwargs):
# 		super().__init__(*args, **kwargs)
# 		self.add_resources(self.additional_resources)

# 	additional_resources = [
# 	]

# if __name__ == "__main__":
	# from .test.blinky import *
