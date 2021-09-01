#!/usr/bin/env python3

#
# This file is part of Tedium.
#
# Copyright (C) 2020 Jared Boone <jared@sharebrained.com>
# SPDX-License-Identifier: BSD-3-Clause

import sys

import array
import struct

import usb.core
import usb.util

from framer_common import *

f = open('/tmp/bob.u8', 'wb')

dev = usb.core.find(idVendor=VENDOR_ID, idProduct=PRODUCT_ID)
# dev.set_configuration()

cfg = dev.get_active_configuration()
# print(cfg)
intf = cfg[(1,1)]
print(intf)
# dev.set_interface_altsetting(interface=1, alternate_setting=1)
# dev.set_interface_altsetting(intf)
intf.set_altsetting()

ep_in = usb.util.find_descriptor(
	intf,
	custom_match=lambda e:
		usb.util.endpoint_direction(e.bEndpointAddress) == usb.util.ENDPOINT_IN
)
# ep_out = usb.util.find_descriptor(
# 	intf,
# 	custom_match=lambda e:
# 		usb.util.endpoint_direction(e.bEndpointAddress) == usb.util.ENDPOINT_OUT
# )

buf = array.array('B', (0,) * 24)

last_count = None

while True:
	n = ep_in.read(buf)
	if n == 24:
		# count = struct.unpack("<I", buf)[0]

		# if last_count is not None:
		# 	diff = count - last_count
		# 	if diff > 1:
		# 		print('.' * (diff - 1))
		# last_count = count
		f.write(buf)
		# print(count)
	elif n == 0:
		print('.')
	else:
		raise RuntimeError('n value unexpected: {}'.format(n))
	# ep_out.write(v)
	# print(v)
