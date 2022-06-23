from luna.gateware.test.utils import LunaGatewareTestCase, sync_test_case

from tedium.gateware.framer.microprocessor import MicroprocessorBus, MicroprocessorInterface

class FramerBusTest(LunaGatewareTestCase):

	FRAGMENT_UNDER_TEST = MicroprocessorInterface
	FRAGMENT_ARGUMENTS = {
		'bus': MicroprocessorBus(),
	}

	SYNC_CLOCK_FREQUENCY = 100e6

	def initialize_signals(self):
		yield self.dut.bus.rdy.eq(0)

	@sync_test_case
	def test_things(self):
		dut: MicroprocessorInterface = self.dut
		bus: MicroprocessorBus = dut.bus

		yield bus.reset.eq(1)

		yield from self.advance_cycles(10)

		yield bus.reset.eq(0)

		# Write operation

		yield dut.address.eq(0x1234)
		yield dut.data_wr.eq(0xaa)
		yield dut.write.eq(1)
		yield dut.start.eq(1)
		self.assertEqual((yield dut.busy), 0)

		yield

		yield dut.start.eq(0)

		for _ in range(25):
			yield
			self.assertEqual((yield dut.busy), 1)

		yield bus.rdy.eq(1)

		yield

		for _ in range(25):
			yield
			self.assertEqual((yield dut.busy), 1)

		yield bus.rdy.eq(0)

		yield

		for _ in range(25):
			yield

		# Read operation

		yield dut.address.eq(0x2345)
		yield dut.write.eq(0)
		yield dut.start.eq(1)
		# self.assertEqual((yield dut.busy), 0)

		yield

		yield dut.start.eq(0)

		for _ in range(25):
			yield
			self.assertEqual((yield dut.busy), 1)

		yield bus.rdy.eq(1)
		yield bus.data.i.eq(0x55)

		yield

		for _ in range(10):
			yield
			# self.assertEqual((yield dut.busy), 0)

		yield bus.rdy.eq(0)

		yield

		for _ in range(10):
			yield
