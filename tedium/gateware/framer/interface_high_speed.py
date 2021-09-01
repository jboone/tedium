#!/usr/bin/env python3

#
# This file is part of Tedium.
#
# Copyright (C) 2020 Jared Boone <jared@sharebrained.com>
# SPDX-License-Identifier: BSD-3-Clause

from nmigen import Elaboratable, Record, Signal, Module, Array

from nmigen.hdl.rec import *
# from nmigen.sim import Delay

# from luna.gateware.test.utils import LunaGatewareTestCase, sync_test_case

# Decided to make HighSpeedBus only use the pins required for a high-speed interface,
# assuming a respin of the boards with fewer connected pins wouldn't have the unused
# pins available as Resource Pins to pass into the bus constructor. It's a hardware
# implementation concern.

class HighSpeedTransmitBus(Record):
	LAYOUT = [
		('ser', [
			('o',  1, DIR_FANOUT),
		]),
		('msync', [
			('i',  1, DIR_FANIN ),
			('o',  1, DIR_FANOUT),
			('oe', 1, DIR_FANOUT),
		]),
		('sync', [
			('i',  1, DIR_FANIN ),
			('o',  1, DIR_FANOUT),
			('oe', 1, DIR_FANOUT),
		]),
	]

	def __init__(self):
		super().__init__(self.LAYOUT)

class HighSpeedReceiveBus(Record):
	LAYOUT = [
		('ser', [
			('i',  1, DIR_FANIN ),
		]),
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
	]

	def __init__(self):
		super().__init__(self.LAYOUT)

class H100Sync(Elaboratable):
	def __init__(self):
		self.enable = Signal()

		# self.inclk = Signal()
		# self.outclk = Signal()

		self.f_bit = Signal()
		self.bit = Signal(range(8))
		self.channel = Signal(range(4))

		self.slot_valid_t1 = Signal()
		self.slot_t1 = Signal(range(32))

		self.slot_valid_e1 = Signal()
		self.slot_e1 = Signal(range(32))

		self.frame_sync = Signal()

	def elaborate(self, platform):
		m = Module()

		# Assuming a frame is composed of 2048 INCLK clocks.
		count = Signal(range(2048))
		count_next = Signal(range(2048))

		with m.If(self.enable):
			m.d.comb += count_next.eq(count + 1)
		with m.Else():
			m.d.comb += count_next.eq(0)

		m.d.sync += [
			count.eq(count_next),
		]

		m.d.comb += [
			self.frame_sync.eq((count == 2047) | (count == 0)),
		]

		slot = Signal(range(32))

		m.d.comb += [
			self.f_bit.eq(count < 8),
			self.bit.eq((count >> 3) & 7),
			self.channel.eq((count & 7) >> 1),

			slot.eq(count >> 6),
		]

		m.d.comb += [
			self.slot_valid_t1.eq((slot & 3) != 0),
			self.slot_t1.eq((slot & 3))
		]

		with m.If(self.slot_valid_t1):
			m.d.comb += self.slot_t1.eq(((slot >> 2) * 3) + ((slot & 3) - 1)),
		with m.Else():
			m.d.comb += self.slot_t1.eq(0)

		m.d.comb += [
			self.slot_valid_e1.eq(1),
			self.slot_e1.eq(slot),
		]

		return m

class HighSpeedTransmit(Elaboratable):
	def __init__(self, bus, sync):
		self.bus = bus
		self.sync = sync

		self.enable = Signal()

		self.data = Array(Signal(8) for _ in range(4))

		# It is the responsibility of the Terminal Equipment to phase-lock the TxSERCLK and
		# TxINCLK to the Recovered Clock of the XRT86VL38 in order to prevent any transmit slip
		# events from occurring.

	def elaborate(self, platform):
		bus = self.bus
		sync = self.sync

		m = Module()

		channel_data = Signal(8)
		m.d.comb += [
			channel_data.eq(self.data[sync.channel]),
		]

		bit = Signal()
		m.d.comb += [
			bit.eq(((channel_data << sync.bit) >> 7) & 1),
		]

		m.d.comb += [
			# FPGA -> framer data
			bus.ser.o.eq(bit & sync.slot_valid_t1),
			bus.ser.oe.eq(self.enable),

			# FPGA -> framer high-speed interface clock (TxINCLKn)
			# bus.msync.o.eq(sync.outclk),
			bus.msync.oe.eq(self.enable),

			# FPGA -> framer F bit (start of frame) marker
			bus.sync.o.eq(sync.frame_sync),
			bus.sync.oe.eq(self.enable),
		]

		return m

def test_h100_sync():
	from nmigen.sim import Simulator, Delay

	bus = HighSpeedTransmitBus()

	m = Module()
	m.submodules.sync = sync = H100Sync()
	m.submodules.dut = dut = HighSpeedTransmit(bus=bus, sync=sync)

	frame_data_0 = [ord(c) for c in '0_TESTING_T1_DATA_STUFF\xff']
	frame_data_1 = [ord(c) for c in '1_TESTING_T1_DATA_STUFF\x00']
	frame_data_2 = [ord(c) for c in '2_TESTING_T1_DATA_STUFF\xff']
	frame_data_3 = [ord(c) for c in '3_TESTING_T1_DATA_STUFF\x00']

	def process_test():
		yield Delay(100e-9)
		yield sync.enable.eq(1)
		yield dut.enable.eq(1)
		yield dut.data[0].eq(0xaa)
		yield dut.data[1].eq(0x55)
		yield dut.data[2].eq(0xff)
		yield dut.data[3].eq(0x00)

		for _ in range(2500):
			slot_t1 = yield sync.slot_t1
			yield
			yield dut.data[0].eq(frame_data_0[slot_t1])
			yield dut.data[1].eq(frame_data_1[slot_t1])
			yield dut.data[2].eq(frame_data_2[slot_t1])
			yield dut.data[3].eq(frame_data_3[slot_t1])

	sim = Simulator(m)
	sim.add_clock(1.0 / 16.384e6)

	# sim.add_sync_process(process_inclk)
	sim.add_sync_process(process_test)

	traces = [
		# sync.inclk,
		# sync.outclk,

		dut.enable,

		bus.ser.o,
		bus.ser.oe,
		bus.msync.o,
		bus.msync.oe,
		bus.sync.o,
		bus.sync.oe,
	]

	with sim.write_vcd("test_h100_sync.vcd", "test_h100_sync.gtkw", traces=traces):
		sim.run()

if __name__ == "__main__":
	test_h100_sync()

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

