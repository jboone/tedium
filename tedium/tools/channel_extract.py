#!/usr/bin/env python3

#
# This file is part of Tedium.
#
# Copyright (C) 2020 Jared Boone <jared@sharebrained.com>
# SPDX-License-Identifier: BSD-3-Clause

import sys

if len(sys.argv) < 2:
	print('incantation: tools/channel_extract.py 0 /tmp/bob_c.u8 | aplay -c 1 -f MU_LAW -r 8000')
	raise RuntimeError('args: <command> <int:channel> [<str:path>]')

channel = int(sys.argv[1])
if len(sys.argv) == 3:
    path_in = sys.argv[2]
else:
    path_in = '/dev/stdin'

READ_SIZE = 24

with open(path_in, 'rb', buffering=READ_SIZE * 16) as i:
    with open('/dev/stdout', 'wb') as o:
        while True:
            sector = i.read(READ_SIZE)
            if len(sector) != READ_SIZE:
                break
            o.write(sector[channel:channel+1])
