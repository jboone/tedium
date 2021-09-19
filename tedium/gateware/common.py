#!/usr/bin/env python3

#
# This file is part of Tedium.
#
# Copyright (C) 2020 Jared Boone <jared@sharebrained.com>
# SPDX-License-Identifier: BSD-3-Clause

import os

VENDOR_ID  = 0x16d0
PRODUCT_ID = 0x0f3b

INTERRUPT_ENDPOINT_NUMBER = 9
MAX_INTERRUPT_PACKET_SIZE = 64 # if os.getenv('LUNA_FULL_ONLY') else 512

ISO_OUT_ENDPOINT_NUMBER = 1
MAX_ISO_OUT_PACKET_SIZE = 24
MAX_ISO_OUT_PACKETS_PER_INTERVAL = 1	# 1: 2 packets/microframe

ISO_IN_ENDPOINT_NUMBER = 2
MAX_ISO_IN_PACKET_SIZE = 24
MAX_ISO_IN_PACKETS_PER_INTERVAL = 1		# 1: 2 packets/microframe

REQUEST_REGISTER_READ     = 0
REQUEST_REGISTER_WRITE    = 1
REQUEST_FRAMER_IF_CONTROL = 2
