#!/usr/bin/env python3

#
# This file is part of Tedium.
#
# Copyright (C) 2020 Jared Boone <jared@sharebrained.com>
# SPDX-License-Identifier: BSD-3-Clause

from nmigen import Elaboratable, Record, Signal, ResetSignal, Module

from nmigen.hdl.rec import *
from nmigen.hdl.ast import Rose, Fell

from nmigen.sim import Delay

from luna.gateware.test.utils import LunaGatewareTestCase, sync_test_case

class ReceiveBus(Record):
	LAYOUT = [
		('sync', [
			('i',  1, DIR_FANIN ),
			('o',  1, DIR_FANOUT),
			('oe', 1, DIR_FANOUT),
		]),
		('crcsync', 1, DIR_FANIN),
		('casync',  1, DIR_FANIN),
		('serclk', [
			('i',  1, DIR_FANIN ),
			('o',  1, DIR_FANOUT),
			('oe', 1, DIR_FANOUT),
		]),
		('ser',  1, DIR_FANIN),
		('sclk', 1, DIR_FANIN),
	]

	def __init__(self):
		super().__init__(self.LAYOUT)

class ReceiveBaseRateStreamBits(Record):
	LAYOUT = [
		('data', 1),

		# Strobe when data has changed.
		('data_strobe', 1),

		# Strobe indicating frame boundary, as determined by framer.
		('frame_strobe', 1),

		# Strobe indicating multiframe boundary, as determined by framer.
		('multiframe_strobe', 1),
	]

	def __init__(self):
		super().__init__(self.LAYOUT)

class ReceiveBaseRate(Elaboratable):
	"""Assumes receive fractional interface is enabled, so recovered clock is output on RxSCLKn."""

	def __init__(self, bus):
		self.bus = bus
		self.stream = ReceiveBaseRateStreamBits()

		# When high, allows FPGA bus to drive (avoid multiple drivers until framer inputs are configured).
		self.enable = Signal()

		# Unsynchronized clock waveform of receiver recovered clock.
		self.recovered_clock = Signal()

	def elaborate(self, platform):
		bus = self.bus
		stream = self.stream

		m = Module()

		# Output SCLK asynchronously, to be passed on to dependent TX blocks.
		m.d.comb += [
			self.recovered_clock.eq(bus.sclk),
		]

		# Configure outputs and output enables.
		# SYNC and SERCLK are driven by framer in this mode.
		m.d.comb += [
			bus.sync.o.eq(0),
			bus.sync.oe.eq(0),
			bus.serclk.o.eq(0),
			bus.serclk.oe.eq(0),
		]

		# Register inputs.
		sync = Signal()
		crcsync = Signal()
		casync = Signal()
		serclk = Signal()
		ser = Signal()
		sclk = Signal()

		m.d.sync += [
			sync.eq(bus.sync.i),
			crcsync.eq(bus.crcsync),
			casync.eq(bus.casync),
			serclk.eq(bus.serclk.i),
			ser.eq(bus.ser),
			sclk.eq(bus.sclk),
		]

		# Falling edge of SERCLK captures other inputs on this interface.
		capture_strobe = Fell(serclk)

		# (E1 mode only) CASYNC output is high during first bit of an E1 CAS multiframe.
		e1_cas_multiframe_strobe = Signal()

		m.d.sync += [
			stream.data_strobe.eq(0),
			stream.frame_strobe.eq(0),
			stream.multiframe_strobe.eq(0),
			e1_cas_multiframe_strobe.eq(0),
		]

		with m.If(capture_strobe):
			m.d.sync += [
				stream.data.eq(ser),
				stream.data_strobe.eq(1),
				stream.frame_strobe.eq(sync),
				stream.multiframe_strobe.eq(crcsync),
				e1_cas_multiframe_strobe.eq(casync),
			]

		return m

class ReceiveBaseRateStreamBytes(Record):
	LAYOUT = [
		# Data byte: first byte in frame contains frame-in-multiframe count.
		('data', 8),

		# Strobe when data has changed.
		('data_strobe', 1),

		# Strobe indicating frame boundary, as determined by framer.
		('frame_strobe', 1),

		# Strobe indicating multiframe boundary, as determined by framer.
		('multiframe_strobe', 1),
	]

	def __init__(self):
		super().__init__(self.LAYOUT)

class DeserializeToBytes(Elaboratable):
	def __init__(self, bus):
		self.bits = ReceiveBaseRateStreamBits()
		self.bytes = ReceiveBaseRateStreamBytes()

	def elaborate(self, platform):
		bits = self.bits
		bytes = self.bytes

		m = Module()

		frame_count = Signal(7)
		frame_count_next = Signal(7)
		with m.If(bits.multiframe):
			m.d.comb += frame_count_next.eq(0)
		with m.Else():
			m.d.comb += frame_count_next.eq(frame_count + 1)

		m.d.sync += [
			bytes.data_strobe.eq(0),
			bytes.frame_strobe.eq(bits.frame_strobe),
			bytes.multiframe_strobe.eq(bits.multiframe_strobe),
		]

		bit_count = Signal(3)
		with m.Elif(bits.frame_strobe):
			# First byte in frame is the frame counter and F bit.
			m.d.sync += [
				bytes.data.eq(Cat(bits.data, frame_count_next)),
				bytes.data_strobe.eq(1),
				bit_count.eq(0),
				frame_count.eq(frame_count_next),
			]
		with m.Elif(bits.data_strobe):
			m.d.sync += [
				bytes.data.eq(Cat(bits.data, bytes.data[1:])),
				bytes.data_strobe.eq(bit_count == 7),
				bit_count.eq(bit_count + 1),
			]

		return m

class TransmitBus(Record):
	LAYOUT = [
		('ser',    1, DIR_FANOUT),
		('serclk', [
			('i',  1, DIR_FANIN ),
			('o',  1, DIR_FANOUT),
			('oe', 1, DIR_FANOUT),
		]),
		('sync', [
			('i',  1, DIR_FANIN ),
			('o',  1, DIR_FANOUT),
			('oe', 1, DIR_FANOUT),
		]),
		('msync', [
			('i',  1, DIR_FANIN ),
			('o',  1, DIR_FANOUT),
			('oe', 1, DIR_FANOUT),
		]),
	]

	def __init__(self):
		super().__init__(self.LAYOUT)

class TransmitStreamBits(Record):
	LAYOUT = [
		# Data input, captured by strobe derived from base_rate_clock.
		('data', 1),

		# Strobe indicating data was accepted by framer (determined by base_rate_clock edge).
		('data_strobe', 1),

		# Strobe indicating frame boundary, as determined by framer.
		('frame_strobe', 1),

		# Strobe indicating multiframe boundary, as determined by framer.
		('multiframe_strobe', 1),
	]

	def __init__(self):
		super().__init__(self.LAYOUT)

class TransmitBaseRate(Elaboratable):
	def __init__(self, bus):
		self.bus = bus
		self.stream = TransmitStreamBits()

		# When high, allows FPGA bus to drive (avoid multiple drivers until framer inputs are configured).
		self.enable = Signal()

		# Unsynchronized clock waveform from receive block, or some other square wave of the appropriate frequency.
		self.base_rate_clock = Signal()

	def elaborate(self, platform):
		bus = self.bus
		stream = self.stream

		m = Module()

		# Configure outputs and output enables.
		m.d.comb += [
			# Output base rate clock (wherever it comes from) to framer's SERCLK pin.
			bus.serclk.o.eq(self.base_rate_clock),
			bus.serclk.oe.eq(self.enable),

			bus.sync.o.eq(0),
			bus.sync.oe.eq(0),

			bus.msync.o.eq(0),
			bus.msync.oe.eq(0),
		]

		# Register inputs.
		serclk = Signal()
		sync = Signal()
		msync = Signal()

		# Synchronize input signals.
		m.d.sync += [
			# Framer SERCLK input is driven by FPGA SERCLK output.
			serclk.eq(bus.serclk.i),
			sync.eq(bus.sync.i),
			msync.eq(bus.msync.i),
		]

		# Capture signals on the rising edge of TxSERCLKn.
		capture_strobe = Fell(serclk)

		m.d.sync += [
			stream.data_strobe.eq(0),
			stream.frame_strobe.eq(0),
			stream.multiframe_strobe.eq(0),
		]

		with m.If(capture_strobe):
			m.d.sync += [
				stream.data_strobe.eq(1),
				stream.frame_strobe.eq(sync),
				stream.multiframe_strobe.eq(msync),
			]

		m.d.comb += [
			bus.ser.eq(stream.data),
		]

		return m

# class FramerBackplane(Elaboratable):
# 	"""HMVIP/H.100 interface"""

# 	def __init__(self, bus):
# 		self.bus = bus

# 	def elaborate(self, platform):
# 		m = Module()

# 		tx0.ser
# 		tx0.
# 		tx0.sync

# 		tx0.serclk
# 		tx1.serclk
# 		tx2.serclk
# 		tx3.serclk

# 		rx0.ser
# 		rx0.serclk
# 		rx0.sync

# 		# RX SCLK always outputs recovered clock when interface is in high-speed/multiplexed modes.
# 		rx0.sclk
# 		rx1.sclk
# 		rx2.sclk
# 		rx3.sclk

# 		tx4.ser
# 		tx4.
# 		tx4.sync

# 		tx4.serclk
# 		tx5.serclk
# 		tx6.serclk
# 		tx7.serclk

# 		rx4.ser
# 		rx4.serclk
# 		rx4.sync

# 		rx4.sclk
# 		rx5.sclk
# 		rx6.sclk
# 		rx7.sclk

# 		m.d.comb += [

# 		]

class T1BaseRateTestCase(LunaGatewareTestCase):

	@staticmethod
	def process_rx_bus(bus, bits):
		frequency = 1.544e6
		period = 1.0 / frequency

		for count, bit in enumerate(bits):
			yield bus.sclk.eq(1)
			yield bus.serclk.eq(1)

			yield Delay(8e-9)

			yield bus.crcsync.eq(count % (193 * 24) == 0)
			yield bus.casync.eq(0)

			yield Delay(2e-9)

			yield bus.ser.eq(int(bit))

			yield Delay(220e-9)

			yield bus.sync.i.eq(count % 193 == 0)

			yield Delay(0.5 * period - 230e9)

			yield bus.sclk.eq(0)
			yield bus.serclk.eq(0)

			yield Delay(0.5 * period)

class ReceiveBaseRateTestCase(T1BaseRateTestCase):

	SYNC_CLOCK_FREQUENCY = 16.384e6

	def instantiate_dut(self):
		self.bus = ReceiveBus()
		return ReceiveBaseRate(bus=self.bus)

	@sync_test_case
	def test_rx_base_rate(self):
		bits_in = '{:193b}'.format(0x10102030405060708090a0b0c0d0e0f101112131415161718)
		bits_in = bits_in * 28

		yield from self.process_rx_bus(self.dut.bus, bits_in)

class TransmitBaseRateTestCase(LunaGatewareTestCase):

	SYNC_CLOCK_FREQUENCY = 16.384e6

	def instantiate_dut(self):
		self.bus = TransmitBus()
		return TransmitBaseRate(bus=self.bus)

	def setUp(self):
		super().setUp()
		self.sim.add_process(self.process_bus)

	def process_bus(self):
		dut = self.dut
		bus = dut.bus

		frequency = 1.544e6
		period = 1.0 / frequency

		tco = 10e-9

		data_out = []

		for count in range(193 * 28):
			yield bus.serclk.i.eq(1)
			
			yield Delay(tco)

			yield bus.sync.i.eq(count % 193 == 0)
			yield bus.msync.i.eq(count % (193 * 24) == 0)
			
			yield Delay(0.5 * period - tco)
			
			yield bus.serclk.i.eq(0)
			
			yield Delay(0.5 * period)

			v = yield bus.ser
			data_out.append(v)
		
		while data_out:
			s = ''.join([str(v) for v in data_out[:193]])
			s_hex = hex(int(s, 2))
			print(s_hex)
			data_out = data_out[193:]

	@sync_test_case
	def test_tx_base_rate(self):
		dut = self.dut
		bus = dut.bus
		stream = dut.stream

		data_in = '{:193b}'.format(0x10102030405060708090a0b0c0d0e0f101112131415161718)
		data_in = data_in * 28

		# data_out = []

		for count, bit in enumerate(data_in):
			yield from self.wait_until(stream.data_strobe)
			yield stream.data.eq(int(bit))
			yield

		# print(data_out)

"""
if __name__ == "__main__":
	from nmigen.sim import Simulator, Delay

	bus = ReceiveBus()

	m = Module()
	m.submodules.dut = dut = ReceiveBaseRate(bus=bus)

	sim = Simulator(m)
	sim.add_clock(1.0 / 12.e6, domain="sync")

	def process():
		data = '{:193b}'.format(0x10102030405060708090a0b0c0d0e0f101112131415161718)
		data = data * 28

		for count, bit in enumerate(data):
			yield dut.bus.sclk.eq(1)
			yield dut.bus.serclk.eq(1)

			if count % 193 == 0:
				yield dut.bus.sync.i.eq(1)
			else:
				yield dut.bus.sync.i.eq(0)

			if count % (193 * 24) == 0:
				yield dut.bus.crcsync.eq(1)
			else:
				yield dut.bus.crcsync.eq(0)

			yield dut.bus.ser.eq(int(bit))

			yield Delay(1.0 / 1.544e6 / 2)

			yield dut.bus.sclk.eq(0)
			yield dut.bus.serclk.eq(0)

			yield Delay(1.0 / 1.544e6 / 2)

	sim.add_process(process)

	traces = [
		dut.bus.sclk,
		dut.bus.serclk.i,
		dut.bus.ser,
		dut.bus.sync.i,
		dut.bus.crcsync,

		dut.stream.data,
		dut.stream.data_strobe,
		dut.stream.frame_strobe,
		dut.stream.multiframe_strobe,
	]

	with sim.write_vcd("test_framer_receive.vcd", "test_framer_receive.gtkw", traces=traces):
		sim.run()
"""

if __name__ == "__main__":
	import unittest

	# To produce simulation output, run as `GENERATE_VCDS=1 <program>`.
	unittest.main()
