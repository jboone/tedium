#!/usr/bin/env python3

#
# This file is part of Tedium.
#
# Copyright (C) 2020 Jared Boone <jared@sharebrained.com>
# SPDX-License-Identifier: BSD-3-Clause

import os

from nmigen                         import Elaboratable, Module, Signal
from usb_protocol.types             import USBTransferType, USBDirection
from usb_protocol.emitters          import DeviceDescriptorCollection

from luna.usb2                      import USBDevice, USBSignalInEndpoint, USBIsochronousInEndpoint

from framer_common import *

class Device(Elaboratable):

	def create_descriptors(self):

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
				i.bInterfaceNumber = 0

				with i.EndpointDescriptor() as e:
					e.bEndpointAddress = USBDirection.IN.to_endpoint_address(INTERRUPT_ENDPOINT_NUMBER)
					e.wMaxPacketSize   = MAX_INTERRUPT_PACKET_SIZE
					e.bmAttributes     = USBTransferType.INTERRUPT

					# Request that we be polled once ber microseconds (2 ^ 3 microframes).
					e.bInterval        = 4

			with c.InterfaceDescriptor() as i:
				i.bInterfaceNumber = 1
				i.bAlternateSetting = 0

			with c.InterfaceDescriptor() as i:
				i.bInterfaceNumber = 1
				i.bAlternateSetting = 1

				with i.EndpointDescriptor() as e:
					e.bEndpointAddress = USBDirection.IN.to_endpoint_address(ISO_ENDPOINT_NUMBER)
					e.wMaxPacketSize   = (TRANSFERS_PER_MICROFRAME << 11) | MAX_ISO_PACKET_SIZE
					e.bmAttributes     = USBTransferType.ISOCHRONOUS

					e.bInterval        = 1

				# with i.EndpointDescriptor() as e:
				# 	e.bEndpointAddress = USBDirection.OUT.to_endpoint_address(ISO_ENDPOINT_NUMBER)
				# 	e.wMaxPacketSize   = (TRANSFERS_PER_MICROFRAME << 11) | MAX_ISO_PACKET_SIZE
				# 	e.bmAttributes     = USBTransferType.ISOCHRONOUS

				# 	e.bInterval        = 1

		return descriptors

	def elaborate(self, platform):
		m = Module()

		# Generate our domain clocks/resets.
		m.submodules.car = platform.clock_domain_generator()

		# Create our USB device interface...
		ulpi = platform.request(platform.default_usb_connection)
		m.submodules.usb = usb = USBDevice(bus=ulpi)

		# Add our standard control endpoint to the device.
		descriptors = self.create_descriptors()
		control_ep = usb.add_standard_control_endpoint(descriptors)

		# Create an interrupt endpoint which will carry the value of our counter to the host
		# each time our interrupt EP is polled.

		# Create the 32-bit counter we'll be using as our status signal.
		counter = Signal(32)
		m.d.usb += counter.eq(counter + 1)

		status_ep = USBSignalInEndpoint(
			width=32,
			endpoint_number=INTERRUPT_ENDPOINT_NUMBER,
			endianness="big"
		)
		usb.add_endpoint(status_ep)
		
		m.d.comb += status_ep.signal.eq(counter)

		# Add a stream endpoint to our device.

		iso_ep = USBIsochronousInEndpoint(
			endpoint_number=ISO_ENDPOINT_NUMBER,
			max_packet_size=MAX_ISO_PACKET_SIZE
		)
		usb.add_endpoint(iso_ep)

		# We'll tie our address directly to our value, ensuring that we always
		# count as each offset is increased.
		m.d.comb += [
			iso_ep.bytes_in_frame.eq(MAX_ISO_PACKET_SIZE),
			iso_ep.value.eq(iso_ep.address)
		]

		# Connect our device as a high speed device by default.
		m.d.comb += [
			usb.connect          .eq(1),
			usb.full_speed_only  .eq(1 if os.getenv('LUNA_FULL_ONLY') else 0),
		]

		return m

if __name__ == "__main__":
	from luna import top_level_cli
	device = top_level_cli(Device)
