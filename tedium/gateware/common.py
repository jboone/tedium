#!/usr/bin/env python3

#
# This file is part of Tedium.
#
# Copyright (C) 2020 Jared Boone <jared@sharebrained.com>
# SPDX-License-Identifier: BSD-3-Clause

CHANNELS = 8
TIMESLOTS_PER_CHANNEL = 24
TIMESLOTS_PER_FRAME = TIMESLOTS_PER_CHANNEL * CHANNELS

REQUEST_REGISTER_READ     = 0
REQUEST_REGISTER_WRITE    = 1
REQUEST_FRAMER_IF_CONTROL = 2
