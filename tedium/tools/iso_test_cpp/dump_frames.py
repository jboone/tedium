#!/usr/bin/env python3

import sys

from bitstring import BitArray

# f = sys.stdin.buffer
f = open(sys.argv[1], "rb")

FRAME_LENGTH = 192 + 2 + 2 + 2 + 2 + 1

f_bits_history = []

print_frames = True
extract_esf_data_link = False

while True:
    frame = f.read(FRAME_LENGTH)
    if len(frame) != FRAME_LENGTH:
        break

    f_bits = int.from_bytes(frame[0:1], byteorder="big")
    if extract_esf_data_link:
        f_bits_history.append(f_bits)

    if print_frames:
        # timeslots = int.from_bytes(frame[0:192], byteorder="big")
        frame_count = int.from_bytes(frame[192:194], byteorder="big")
        usb_in_clock_diff = int.from_bytes(frame[194:196], byteorder="big")
        rx_fifo_level = int.from_bytes(frame[196:198], byteorder="big")
        tx_fifo_level = int.from_bytes(frame[198:200], byteorder="big")
        flags = int.from_bytes(frame[200:201], byteorder="big")

        # frame_int = int.from_bytes(frame, byteorder='big')
        # print(f"{timeslots:0384x} {frame_count:04x} {usb_in_clock_diff:04x} {rx_fifo_level:04x} {tx_fifo_level:04x} {flags:08b}")
        # print(f"{frame_count:04x} {usb_in_clock_diff:04x} {rx_fifo_level:04x} {tx_fifo_level:04x} {flags:08b}")
        oops = "*" if rx_fifo_level > 1000 else " "
        print(f"{frame_count:04x} {usb_in_clock_diff:04x} {rx_fifo_level:04x}{oops}{tx_fifo_level:04x} {flags:08b}")

if extract_esf_data_link:
    f_bits_history = bytearray(f_bits_history)
    f_bits_history = BitArray(bytes=f_bits_history)

    for channel in range(8):
        print(f"channel {channel}")
        f_bits = f_bits_history[7 - channel::8]

        tolerance = 0.01

        esf_pattern_search = {
            "001011": 0,
            "010110": 1,
            "101100": 2,
            "011001": 3,
            "110010": 4,
            "100101": 5,
        }

        patterns = {}

        for p1 in range(4):
            pattern = []

            for p2 in range(6):
                phase = (p2 << 2) | p1
                phase_bits = f_bits[phase::24]
                average = sum(phase_bits) / len(phase_bits)

                if average < tolerance:
                    pattern.append("0")
                elif average > (1 - tolerance):
                    pattern.append("1")
                else:
                    pattern.append("X")

            pattern = ''.join(pattern)

            if 'X' not in pattern:
                patterns[p1] = pattern

        phase = None
        for p1, pattern in patterns.items():
            if pattern in esf_pattern_search:
                p2 = esf_pattern_search[pattern]
                phase = (p2 << 2) | p1
                break

        if phase is not None:
            f_bits_framed = f_bits[phase:]
            esf_sync = f_bits_framed[0::4]
            crc = f_bits_framed[2::4]
            fdl = f_bits_framed[1::2]

            # print(crc.bin)
            delimiter = BitArray(bin="11111111")
            print([code.bin for code in fdl.split(delimiter)])

    # FDL
    #
    # Analog Loopback inside Tedium, 2022/Apr/02, due to troubles with one Adit channel not going green.
    # Channel 0: 11111111 10111110 11111111 10111110
    # Channel 1: 11111111 00101000 11111111 00101000
    # Channel 2: 11111111 10101100 11111111 10101100
    #
    # From the T-BERD 950 in SIG+BERT mode
    # ...all zeros.
    # 
    # From the Adits:
    # "01111110" (HDLC idle) interspersed with:
    # 000111001000000011000000000000001100000000000000010000000000000010000000000000000000000011010101011111000
    # 00011100100000001100000000000000000000000000000011000000000000000100000000000000100000000011011011000110
    # 00011100100000001100000000000000100000000000000000000000000000001100000000000000010000000100110010010001
    # 00011100100000001100000000000000010000000000000010000000000000000000000000000000110000001010111100101011
    # 000111001000000011000000000000001100000000000000010000000000000010000000000000000000000011010101011111000
    #
    # 000111001000000011000000000000001100000000000000010000000000000010000000000000000000000011010101011111000
    # 00011100100000001100000000000000000000000000000011000000000000000100000000000000100000000011011011000110
    # 00011100100000001100000000000000100000000000000000000000000000001100000000000000010000000100110010010001
    # 00011100100000001100000000000000010000000000000010000000000000000000000000000000110000001010111100101011
    # 000111001000000011000000000000001100000000000000010000000000000010000000000000000000000011010101011111000
    #
    # And now via analog loopback inside Tedium, once I turned on MOS signaling on HDLC1. (Why the hell does
    # one Adit-connected Tedium channel decide to stop sending performance reports when loopback is entered?)
    #
    # 00011100100000001100000001000010100000000100001000000000010000101100000001000010010000000011110011101101
    # 000111001000000011000000010000100100000001000010100000000100001000000000010000101100000011011111001010111
    # 00011100100000001100000001000010110000000100001001000000010000101000000001000010000000001010010100000000
