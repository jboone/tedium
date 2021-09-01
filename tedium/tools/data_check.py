#!/usr/bin/env python3

#
# This file is part of Tedium.
#
# Copyright (C) 2020 Jared Boone <jared@sharebrained.com>
# SPDX-License-Identifier: BSD-3-Clause

import sys

import numpy

d = numpy.fromfile(sys.argv[1], dtype=numpy.uint8)
d = numpy.reshape(d[:(len(d) // 24) * 24], (-1, 24))

v = numpy.uint8(d[0][0] + 255)

for row in d:
	v = numpy.uint8(v + 1)
	if numpy.all(row == v):
		print(v, row[0])
		# pass
	else:
		print(v, row)
		v = numpy.uint8(row[0])
