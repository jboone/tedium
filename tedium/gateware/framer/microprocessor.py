#!/usr/bin/env python3

#
# This file is part of Tedium.
#
# Copyright (C) 2020 Jared Boone <jared@sharebrained.com>
# SPDX-License-Identifier: BSD-3-Clause

from amaranth import Elaboratable, Record, Signal, Module

from amaranth.hdl.rec import DIR_FANIN, DIR_FANOUT

class MicroprocessorBus(Record):
	""" Record representing an XRT86VX38 TDM framer/LIU microprocessor interface. """

	LAYOUT = [
		('addr',  15, DIR_FANOUT),
		('data', [
			('i',  8, DIR_FANIN),
			('o',  8, DIR_FANOUT),
			('oe', 1, DIR_FANOUT),
		]),
		('pclk',   1, DIR_FANOUT),
		('cs',     1, DIR_FANOUT),
		('ale',    1, DIR_FANOUT),
		('rd',     1, DIR_FANOUT),
		('wr',     1, DIR_FANOUT),
		('rdy',    1, DIR_FANIN),
		('int',    1, DIR_FANIN),
		('req',    2, DIR_FANIN),
		('ack',    2, DIR_FANOUT),
		('ptype0', 1, DIR_FANOUT),
		('ptype2', 1, DIR_FANOUT),
		('reset',  1, DIR_FANOUT),
	]

	def __init__(self):
		super().__init__(self.LAYOUT)

class MicroprocessorInterface(Elaboratable):
	""" XRT86VX38 Framer/LIU processor bus

    NOTE: Be sure to DomainRename "sync" to "usb"!
	"""
	def __init__(self, *, bus: MicroprocessorBus):
		self.bus     = bus

		self.address = Signal(15)
		self.data_wr = Signal(8)
		self.data_rd = Signal(8)

		self.start   = Signal()
		self.write   = Signal()
		self.busy    = Signal()
		self.done    = Signal()

		self.cycles  = Signal(8)

		self._wait_cycles = 20
		
		# Using Intel uP asynchronous interface mode with ALE=1 (high), which removes ALE-related timing requirements.
		# Timing from XRT86VX38 Framer/LIU hardware description.
		# Figure 11 "Intel uP Interface Timing During Programmed I/O Read and Write Operations When ALE Is Tied 'HIGH'"

	def elaborate(self, platform):
		m = Module()

		m.d.comb += [
			self.bus.pclk.eq(0),	# PCLK is unused in Intel uP mode.
			self.bus.ale.eq(1),		# Use ALE=1 Intel uP interface variant, which simplifies state management and timing.
			self.bus.ack.eq(0b00),	# TODO: Implement DMA.
			self.bus.ptype0.eq(0),	# PTYPE[2,0]: Configure interface for Intel uP interface mode.
			self.bus.ptype2.eq(0),	# PTYPE1 is wired to ground/low/0.
		]

		m.d.sync += self.done.eq(0)

		with m.FSM(domain="sync", reset="IDLE") as fsm:
			with m.State("IDLE"):
				with m.If(self.start):
					m.d.sync += [
						self.busy.eq(1),
						self.bus.addr.eq(self.address),
					]

					m.next = 'CS-ASSERT'

			with m.State("CS-ASSERT"):
				m.d.sync += [
					self.bus.cs.eq(1),
					self.bus.data.o.eq(self.data_wr),
					self.bus.data.oe.eq(self.write),
					self.cycles.eq(self._wait_cycles),
				]

				m.next = 'RD-WR-ASSERT'

			with m.State('RD-WR-ASSERT'):
				# It sounds like the RD or WR pulse needs to be >320 ns.
				# I wish the datasheet were a bit more clear...
				with m.If(self.cycles > 0):
					m.d.sync += [
						self.cycles.eq(self.cycles - 1),
					]
				with m.Else():
					m.d.sync += [
						self.bus.rd.eq(~self.write),
						self.bus.wr.eq(self.write),
						self.cycles.eq(self._wait_cycles),
					]

					m.next = 'RDY-WAIT'

			with m.State('RDY-WAIT'):
				with m.If(self.cycles > 0):
					m.d.sync += [
						self.cycles.eq(self.cycles - 1),
					]
				with m.Elif(self.bus.rdy):
					m.d.sync += [
						self.data_rd.eq(self.bus.data.i),
						self.bus.cs.eq(0),
						self.bus.rd.eq(0),
						self.bus.wr.eq(0),
						self.bus.data.oe.eq(0),
					]

					m.next = 'DONE-WAIT'

			with m.State('DONE-WAIT'):
				# TODO: There's certainly a minimum CS# deasserted period that isn't reflected in
				# the datasheet...
				with m.If(~self.bus.rdy):
					m.d.sync += [
						self.busy.eq(0),
						self.done.eq(1),
					]

					m.next = 'IDLE'

		return m
