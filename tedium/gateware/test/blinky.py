#!/usr/bin/env python3

#
# This file is part of Tedium.
#
# Copyright (C) 2020 Jared Boone <jared@sharebrained.com>
# SPDX-License-Identifier: BSD-3-Clause

# Heavily borrowed from the LUNA project.

import sys
import os
import logging
import time

import usb1

from nmigen                        import Elaboratable, Module, Signal
from usb_protocol.emitters         import DeviceDescriptorCollection

# from luna        import top_level_cli
from luna.usb2   import USBDevice, USBStreamInEndpoint

from boards import *

VENDOR_ID  = 0x16d0
PRODUCT_ID = 0x0f3b

class SpeedTest(Elaboratable):

	BULK_ENDPOINT_NUMBER = 1
	MAX_BULK_PACKET_SIZE = 512 # 64 if os.getenv('LUNA_FULL_ONLY') else 512

	def create_descriptors(self): #, high_speed=True):

		descriptors = DeviceDescriptorCollection()

		with descriptors.DeviceDescriptor() as d:
			d.idVendor  = VENDOR_ID
			d.idProduct = PRODUCT_ID

			d.iManufacturer = "ShareBrained"
			d.iProduct      = "Tedium X8"
			d.iSerialNumber = "deadbeef"

			d.bNumConfigurations = 1

		with descriptors.ConfigurationDescriptor() as c:
			with c.InterfaceDescriptor() as i:
				i.bInterfaceNumber = 0;

				# OUT descriptor
				# with i.EndpointDescriptor() as e:
				# 	e.bEndpointAddress = self.BULK_ENDPOINT_NUMBER | 0x00
				# 	e.wMaxPacketSize = self.MAX_BULK_PACKET_SIZE

				# IN descriptor
				with i.EndpointDescriptor() as e:
					e.bEndpointAddress = self.BULK_ENDPOINT_NUMBER | 0x80
					e.wMaxPacketSize = self.MAX_BULK_PACKET_SIZE

		return descriptors

	def elaborate(self, platform):
		m = Module()

		m.submodules.car = platform.clock_domain_generator()

		ulpi = platform.request('ulpi')
		m.submodules.usb = usb = USBDevice(bus=ulpi)

		descriptors = self.create_descriptors()
		usb.add_standard_control_endpoint(descriptors)

		stream_ep = USBStreamInEndpoint(
			endpoint_number=self.BULK_ENDPOINT_NUMBER,
			max_packet_size=self.MAX_BULK_PACKET_SIZE,
		)
		usb.add_endpoint(stream_ep)

		counter = Signal(8)
		with m.If(stream_ep.stream.ready):
			m.d.usb += counter.eq(counter + 1)

		m.d.comb += [
			stream_ep.stream.valid  .eq(1),
			stream_ep.stream.payload.eq(counter),
		]

		m.d.comb += [
			usb.connect        	.eq(1),
			usb.full_speed_only	.eq(0),
		]

		return m

# Set the total amount of data to be used in our speed test.
TEST_DATA_SIZE = 1 * 1024 * 1024
TEST_TRANSFER_SIZE = 16 * 1024

# Size of the host-size "transfer queue" -- this is effectively the number of async transfers we'll
# have scheduled at a given time.
TRANSFER_QUEUE_DEPTH = 16 #* 2 # * 48

def run_speed_test():
	""" Runs a simple speed test, and reports throughput. """

	total_data_exchanged = 0
	failed_out = False

	f_out = open('test.bin', 'wb')

	_messages = {
		1: "error'd out",
		2: "timed out",
		3: "was prematurely cancelled",
		4: "was stalled",
		5: "lost the device it was connected to",
		6: "sent more data than expected."
	}

	def _should_terminate():
		""" Returns true iff our test should terminate. """
		return (total_data_exchanged > TEST_DATA_SIZE) or failed_out


	def _transfer_completed(transfer: usb1.USBTransfer):
		""" Callback executed when an async transfer completes. """
		nonlocal total_data_exchanged, failed_out

		status = transfer.getStatus()

		# If the transfer completed.
		if status in (usb1.TRANSFER_COMPLETED,):

			actual_length = transfer.getActualLength()

			f_out.write(transfer.getBuffer()[:actual_length])

			# Count the data exchanged in this packet...
			total_data_exchanged += actual_length

			# ... and if we should terminate, abort.
			if _should_terminate():
				return

			# Otherwise, re-submit the transfer.
			transfer.submit()

		else:
			failed_out = status



	with usb1.USBContext() as context:

		# Grab a reference to our device...
		device = context.openByVendorIDAndProductID(VENDOR_ID, PRODUCT_ID)

		# ... and claim its bulk interface.
		device.claimInterface(0)

		# Submit a set of transfers to perform async comms with.
		active_transfers = []
		for _ in range(TRANSFER_QUEUE_DEPTH):

			# Allocate the transfer...
			transfer = device.getTransfer()
			transfer.setBulk(0x80 | SpeedTest.BULK_ENDPOINT_NUMBER, TEST_TRANSFER_SIZE, callback=_transfer_completed, timeout=1000)

			# ... and store it.
			active_transfers.append(transfer)


		# Start our benchmark timer.
		start_time = time.time()

		# Submit our transfers all at once.
		for transfer in active_transfers:
			transfer.submit()

		# Run our transfers until we get enough data.
		while not _should_terminate():
			context.handleEvents()

		# Figure out how long this took us.
		end_time = time.time()
		elapsed = end_time - start_time

		# Cancel all of our active transfers.
		for transfer in active_transfers:
			if transfer.isSubmitted():
				transfer.cancel()

		# If we failed out; indicate it.
		if (failed_out):
			logging.error(f"Test failed because a transfer {_messages[failed_out]}.")
			sys.exit(failed_out)


		bytes_per_second = total_data_exchanged / elapsed
		bits_per_second = bytes_per_second * 8
		logging.warning(f"Exchanged {total_data_exchanged / 1000000}MB total at {bytes_per_second / 1000000}MB/s ({bits_per_second / 1000000}Mb/s).")

# Log formatting strings.
LOG_FORMAT_COLOR = "\u001b[37;1m%(levelname)-8s| \u001b[0m\u001b[1m%(module)-12s|\u001b[0m %(message)s"
LOG_FORMAT_PLAIN = "%(levelname)-8s:n%(module)-12s>%(message)s"

if __name__ == "__main__":
	# top_level_cli(SpeedTest)


	# I am sure there's a better, magical way to do this.
	activity = 'build'

	if activity == 'build':
		platform = TediumX8Platform()
		platform.build(SpeedTest(), do_program=True)
	elif activity == 'run':
		print('Running...')
		if sys.stdout.isatty():
			log_format = LOG_FORMAT_COLOR
		else:
			log_format = LOG_FORMAT_PLAIN

		# TODO: I have NO idea how Python logging works. Despite setting
		# the parameters I want, the changes don't take effect. It's probably
		# due to Luna magic.

		# logging.basicConfig(level=logging.INFO, format=log_format)

		logging.warning("Giving the device time to connect...")
		time.sleep(5)

		logging.warning(f"Starting bulk in speed test.")
		run_speed_test()