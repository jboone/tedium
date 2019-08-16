// Copyright 2019 Jared Boone
// (Hi, Apache license, am I doing this right?)

use std::fs;

struct Bits<'a> {
	vec: &'a Vec<u8>,
	idx_now: usize,
	idx_end: usize,
}


impl<'a> Bits<'a> {
	fn new(vec: &'a Vec<u8>) -> Bits {
		Bits { vec: vec, idx_now: 0, idx_end: vec.len() * 8 }
	}
}

impl<'a> Iterator for Bits<'a> {
	type Item = u32;

	#[inline]
	fn next(&mut self) -> Option<u32> {
		if self.idx_now == self.idx_end {
			None
		} else {
			let byte_idx = self.idx_now >> 3;
			let bit_idx = (self.idx_now ^ 7) & 7;
			let value = (self.vec[byte_idx] >> bit_idx) as u32 & 1;
			self.idx_now += 1;
			Some(value)
		}
	}
}

const FRAME_LENGTH: usize = 193;
const ESF_FRAME_COUNT: usize = 24;
// const ESF_LENGTH: usize = ESF_FRAME_COUNT * FRAME_LENGTH;

const FRAMING_FAS_MASK: u32 = 0b0001_0001_0001_0001_0001_0001;
const FRAMING_FAS_MATCH: u32 = 0b0000_0000_0001_0000_0001_0001;
const FRAMING_FAS_MATCH_THRESHOLD: u8 = 100;

#[derive(Clone, Copy)]
enum State {
	Sync,
	Up,
}

fn main() {
	let bytes = fs::read("test.u8").unwrap();
	let source = Bits::new(&bytes);

	let mut bits = [0u32; 193];
	let mut stats = [0u8;  193];
	let mut bit_n = 0;
	let mut frame_n = 0;
	let mut fas = 0u32;
	let mut crc = 0u32;
	let mut dl = 0u32;
	let mut state = State::Sync;
	let mut state_next = State::Sync;
	let mut channel = [0u8; 24];
	let mut remainder = 0u32;
	let mut crc_expected = 0u32;

	for bit in source {
		match state {
			State::Sync => {
				bits[bit_n] = (bits[bit_n] << 1) | bit;
				if bits[bit_n] & FRAMING_FAS_MASK == FRAMING_FAS_MATCH {
					stats[bit_n] += 1;
					if stats[bit_n] == FRAMING_FAS_MATCH_THRESHOLD {
						state_next = State::Up;
						bit_n = 0;
						frame_n = ESF_FRAME_COUNT - 1;
					}
				}
			},
			State::Up => {
				if bit_n == 0 {
					if frame_n == 0 {
						dl  &= 0xfff;
						fas &= 0x3f;
						crc &= 0x3f;
						crc_expected &= 0x3f;
						let crc_ok = crc == crc_expected;
						let crc_status = match crc_ok {
							true => "OK",
							false => "fail",
						};
						println!("frame(fas={:06b}, crc={:06b}/{:06b} {}, dl={:012b})", fas, crc, crc_expected, crc_status, dl);
						crc_expected = remainder;
						remainder = 0;
					}

					// Handle F bit
					if (frame_n & 1) == 0 {
						dl  = (dl  << 1) | bit;
					} else {
						if (frame_n & 3) == 3 {
							fas = (fas << 1) | bit;
						}
						if (frame_n & 3) == 1 {
							crc = (crc << 1) | bit;
						}
					}
				} else {
					let channel_n = (bit_n - 1) >> 3;
					channel[channel_n] <<= 1;
					channel[channel_n] |= bit as u8;
				}

				if (bit != 0) || (bit_n == 0) {
					remainder ^= 0x20;
				}
				let do_poly_div = match remainder & 0x20 {
					0 => false,
					_ => true,
				};
				remainder <<= 1;
				if do_poly_div {
					remainder ^= 0x03;
				}
			}
		}

		bit_n += 1;
		if bit_n == FRAME_LENGTH {
			bit_n = 0;
			frame_n += 1;
			if frame_n == ESF_FRAME_COUNT {
				state = state_next;
				frame_n = 0;
			}
		}
	}
}
