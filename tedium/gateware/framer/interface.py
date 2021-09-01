#!/usr/bin/env python3

#
# This file is part of Tedium.
#
# Copyright (C) 2020 Jared Boone <jared@sharebrained.com>
# SPDX-License-Identifier: BSD-3-Clause

from nmigen import Record, Elaboratable, Signal, Module, Array
from nmigen.hdl.rec import DIR_FANIN, DIR_FANOUT

class BaseRateReceiveBus(Record):
	LAYOUT = [
		('sclk', [
			('i',  1, DIR_FANIN ),
		]),
		('serclk', [
			('o',  1, DIR_FANOUT),
			('oe', 1, DIR_FANOUT),
		]),
		('ser', [
			('i',  1, DIR_FANIN ),
		]),
		('sync', [
			('o',  1, DIR_FANOUT),
			('oe', 1, DIR_FANOUT),
		]),
	]

	def __init__(self):
		super().__init__(self.LAYOUT)

class BaseRateReceiver(Elaboratable):

	def __init__(self, bus):
		self.bus = bus

		# in: Enables output drivers connected to framer.
		self.enable = Signal()

		# in: Start of frame (true for one base rate clock period).
		self.start_of_frame = Signal()

		# in: Unsynchronized clock for framer data transfer interface.
		self.base_rate_clock = Signal()

		# in: Synchronized strobe (one sync clock duration) for capturing received data.
		self.capture_strobe = Signal()

		# out: Unsynchronized recovered clock from receiver LIU.
		self.recovered_clock = Signal()

		# out: Framer data captured by capture_strobe.
		self.data = Signal()

	def elaborate(self, platform):
		m = Module()

		bus = self.bus

		# Configure outputs.
		m.d.comb += [
			# Send SERCLK interface clock to framer.
			bus.serclk.o.eq(self.base_rate_clock),
			bus.serclk.oe.eq(self.enable),

			# Send SYNC frame sync to framer. Slip buffer will align for us.
			bus.sync.o.eq(self.start_of_frame),
			bus.sync.oe.eq(self.enable),
		]

		# Pass SCLK recovered clock out of this module.
		m.d.comb += [
			self.recovered_clock.eq(bus.sclk),
		]

		# Capture SER data.
		with m.If(self.capture_strobe):
			m.d.sync += self.data.eq(bus.ser)

		return m

class BaseRateTransmitBus(Record):
	LAYOUT = [
		('serclk', [
			('o',  1, DIR_FANOUT),
			('oe', 1, DIR_FANOUT),
		]),
		('ser', [
			('o',  1, DIR_FANOUT),
			('oe', 1, DIR_FANOUT),
		]),
		('sync', [
			('o',  1, DIR_FANOUT),
			('oe', 1, DIR_FANOUT),
		]),
		('msync', [
			('o',  1, DIR_FANOUT),
			('oe', 1, DIR_FANOUT),
		]),
	]

	def __init__(self):
		super().__init__(self.LAYOUT)

class BaseRateTransmitter(Elaboratable):

	def __init__(self, bus):
		self.bus = bus

		# in: Enables output drivers connected to framer.
		self.enable = Signal()

		# in: Data to be transmitted to framer.
		self.data = Signal()

		# in: Start of frame (true for one base rate clock period).
		self.start_of_frame = Signal()

		# in: Start of multiframe (true for one base rate clock period).
		self.start_of_multiframe = Signal()

		# in: Unsynchronized clock for framer data transfer interface.
		self.base_rate_clock = Signal()

	def elaborate(self, platform):
		m = Module()

		bus = self.bus

		# Configure outputs
		m.d.comb += [
			# Transmit data.
			bus.ser.eq(self.data),

			# Timing source of transmit line interface.
			bus.serclk.o.eq(self.base_rate_clock),
			bus.serclk.oe.eq(self.enable),

			# Transmit frame sync pulse.
			bus.sync.o.eq(self.start_of_frame),
			bus.sync.oe.eq(self.enable),

			# Transmit multiframe sync pulse.
			bus.msync.o.eq(self.start_of_multiframe),
			bus.msync.oe.eq(self.enable),
		]

		return m

class BaseRateSync(Elaboratable):
	def __init__(self):
		self.strobe_in = Signal()
		self.strobe_out = Signal()

		self.start_of_frame = Signal()
		self.end_of_frame = Signal()

		self.start_of_multiframe = Signal()
		self.end_of_multiframe = Signal()

		self.slot = Signal(range(32))
		self.start_of_slot = Signal()
		self.end_of_slot = Signal()

		self.f = Signal()
		self.payload = Signal()

		self.bit = Signal(range(8))
		self.start_of_bit = Signal()
		self.end_of_bit = Signal()

	def elaborate(self, platform):
		m = Module()

		MULTIFRAME_LENGTH = 24
		FRAME_LENGTH = 193

		m.d.sync += self.strobe_out.eq(self.strobe_in)

		m.d.comb += self.end_of_bit.eq(self.strobe_in)

		m.d.sync += self.start_of_bit.eq(self.end_of_bit)

		###############################################################
		# Bit counter.

		count = Signal(range(FRAME_LENGTH))
		count_next = Signal.like(count)

		with m.If(self.end_of_frame):
			m.d.comb += count_next.eq(0)
		with m.Else():
			m.d.comb += count_next.eq(count + 1)

		with m.If(self.strobe_in):
			m.d.sync += count.eq(count_next)

		###############################################################
		# Frame counter.

		end_of_frame_next = count_next == (FRAME_LENGTH - 1)

		frame_count = Signal(range(MULTIFRAME_LENGTH))

		with m.If(self.strobe_in):
			m.d.sync += [
				self.start_of_multiframe.eq(self.end_of_multiframe),
				self.end_of_multiframe.eq(0),
			]

			with m.If(end_of_frame_next):
				with m.If(frame_count == (MULTIFRAME_LENGTH - 1)):
					m.d.sync += self.end_of_multiframe.eq(1),

			with m.If(self.end_of_frame):
				with m.If(self.end_of_multiframe):
					m.d.sync += frame_count.eq(0)
				with m.Else():
					m.d.sync += frame_count.eq(frame_count + 1)

		###############################################################
		# Event signals.

		with m.If(self.strobe_in):
			m.d.sync += [
				self.start_of_frame.eq(count_next == 0),
				self.f.eq(count_next == 0),
				self.payload.eq(count_next != 0),
				self.end_of_frame.eq(end_of_frame_next),
				self.bit.eq(~(count[0:3])),
				self.start_of_slot.eq((self.end_of_slot ^ self.end_of_frame) | self.f),
				self.end_of_slot.eq(self.bit == 1),
			]

			with m.If(self.end_of_frame):
				m.d.sync += self.slot.eq(0),
			with m.Else():
				m.d.sync += self.slot.eq(count[3:]),

		return m

def test_base_rate_sync():
	from nmigen.sim import Simulator, Delay

	clock_sclk = 1.544e6
	clock_sync = 16.384e6

	m = Module()
	m.submodules.dut = dut = BaseRateSync()

	sclk = Signal()
	serclk = Signal()
	ser = Signal()

	SERCLK_SKEW = 10e-9
	SER_SKEW = 10e-9

	def process_framer():
		frequency = clock_sclk
		period = 1.0 / frequency

		data = 'THIS_IS_A_TEST_' * 40
		data_bits = ''.join(['{:08b}'.format(ord(v)) for v in data])

		for bit in data_bits:
			yield sclk.eq(1)
			yield Delay(SERCLK_SKEW)
			yield serclk.eq(1)
			yield Delay(SER_SKEW)
			yield ser.eq(int(bit))
			yield Delay(period * 0.5 - SERCLK_SKEW - SER_SKEW)
			yield sclk.eq(0)
			yield Delay(SERCLK_SKEW)
			yield serclk.eq(0)
			yield Delay(period * 0.5 - SERCLK_SKEW)

	def process_strobe():
		last = 0
		for _ in range(int(round(4700 * clock_sync / clock_sclk))):
			serclk_value = yield serclk
			if serclk_value == 0 and last == 1:
				yield dut.strobe_in.eq(1)
			else:
				yield dut.strobe_in.eq(0)
			last = serclk_value
			yield

	def process_test():
		yield Delay(100e-9)

		for _ in range(4700):
			yield

	sim = Simulator(m)
	sim.add_clock(1.0 / clock_sync)

	sim.add_process(process_framer)
	sim.add_sync_process(process_strobe)
	sim.add_sync_process(process_test)

	traces = [
		sclk,
		serclk,
		ser,
	]

	with sim.write_vcd("test_base_rate_sync.vcd", "test_base_rate_sync.gtkw", traces=traces):
		sim.run()

if __name__ == "__main__":
	test_base_rate_sync()
