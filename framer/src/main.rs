// Copyright 2019 Jared Boone
// (Hi, Apache license, am I doing this right?)

use std::fs;
// use std::fs::File;

// use std::io::Write;

// #[derive(Copy, Clone)]
struct Bits {
	vec: Vec<u8>,
	idx_now: usize,
	idx_end: usize,
}


impl Bits {
	fn new(d: &[u8]) -> Bits {
		Bits { vec: d.to_vec(), idx_now: 0, idx_end: d.len() * 8 }
	}
}

impl Iterator for Bits {
	type Item = bool;

	#[inline]
	fn next(&mut self) -> Option<bool> {
		if self.idx_now == self.idx_end {
			None
		} else {
			let byte_idx = self.idx_now >> 3;
			let bit_idx = (self.idx_now ^ 7) & 7;
			let value = (self.vec[byte_idx] >> bit_idx) & 1;
			self.idx_now += 1;
			Some(value != 0)
		}
	}
}

struct BitAccumulator {
	value: u32,
	width: usize,
}

impl BitAccumulator {
	fn new(width: usize) -> BitAccumulator {
		BitAccumulator {
			value: 0,
			width,
		}
	}

	fn update(&mut self, bit: bool) {
		self.value = (self.value << 1) | (bit as u32);
	}

	fn mask(&self) -> u32 {
		(1 << self.width) - 1
	}

	fn value(&self) -> u32 {
		self.value & self.mask()
	}
}

struct CRC {
	remainder: u32,
	poly: u32,
	width: usize,
}

impl CRC {
	fn new_crc6() -> CRC {
		CRC {
			remainder: 0,
			poly: 0x03,
			width: 6,
		}
	}

	fn reset(&mut self) {
		self.remainder = 0;
	}

	fn top_bit(&self) -> u32 {
		1 << (self.width - 1)
	}

	fn update(&mut self, bit: bool) {
		self.remainder ^= self.top_bit() * (bit as u32);
		let do_poly_div = match self.remainder & self.top_bit() {
			0 => false,
			_ => true,
		};
		self.remainder <<= 1;
		if do_poly_div {
			self.remainder ^= self.poly;
		}
	}

	fn finalize(&self) -> u32 {
		self.remainder & ((1 << self.width) - 1)
	}
}
/*
// #[derive(Clone, Copy)]
enum State {
	Sync(ExtendedSuperFrameSync),
	Up(ExtendedSuperFrameDemux),
}
*/
trait BitSink {
	// fn update(&mut self, bit: bool) -> State;
	fn update(&mut self, bit: bool);
}

const FRAME_LENGTH: usize = 193;
const ESF_FRAME_COUNT: usize = 24;
// const ESF_LENGTH: usize = ESF_FRAME_COUNT * FRAME_LENGTH;

const FRAMING_FAS_MASK: u32 = 0b0001_0001_0001_0001_0001_0001;
const FRAMING_FAS_MATCH: u32 = 0b0000_0000_0001_0000_0001_0001;
const FRAMING_FAS_MATCH_THRESHOLD: u8 = 100;

struct ExtendedSuperFrameDemux {
	bit_n: usize,
	frame_n: usize,
	fas: BitAccumulator,
	crc: BitAccumulator,
	dl: BitAccumulator,
	channel: [u8; 24],
	crc6: CRC,
	crc_expected: u32,
}

impl ExtendedSuperFrameDemux {
	fn new() -> ExtendedSuperFrameDemux {
		ExtendedSuperFrameDemux {
			bit_n: 0,
			frame_n: 0,
			fas: BitAccumulator::new(6),
			crc: BitAccumulator::new(6),
			dl: BitAccumulator::new(12),
			channel: [0u8; 24],
			crc6: CRC::new_crc6(),
			crc_expected: 0u32,
		}
	}

	fn synchronize(&mut self, bit_n: usize, frame_n: usize) {
		self.bit_n = bit_n;
		self.frame_n = frame_n;
	}

	fn update(&mut self, bit: bool) {
		let f_bit = self.bit_n == 0;

		if f_bit {
			if self.frame_n == 0 {
				let crc_ok = self.crc.value() == self.crc_expected;
				let crc_status = match crc_ok {
					true => "OK",
					false => "fail",
				};
				println!("frame(fas={:06b}, crc={:06b}/{:06b} {}, dl={:012b})",
					self.fas.value(), self.crc.value(), self.crc_expected, crc_status, self.dl.value());
				self.crc_expected = self.crc6.finalize();
				self.crc6.reset();
			}

			// file_channels.write_all(&channel).unwrap();

			// Handle F bit
			if (self.frame_n & 1) == 0 {
				self.dl.update(bit);
			} else {
				if (self.frame_n & 3) == 3 {
					self.fas.update(bit);
				}
				if (self.frame_n & 3) == 1 {
					self.crc.update(bit);
				}
			}
		} else {
			let channel_n = (self.bit_n - 1) >> 3;
			self.channel[channel_n] <<= 1;
			self.channel[channel_n] |= bit as u8;
		}

		let crc_bit = bit || f_bit;
		self.crc6.update(crc_bit);

		self.bit_n += 1;
		if self.bit_n == FRAME_LENGTH {
			self.bit_n = 0;
			self.frame_n += 1;
			if self.frame_n == ESF_FRAME_COUNT {
				// state = state_next;
				self.frame_n = 0;
			}
		}
	}
}

struct ExtendedSuperFrameSync {
	bit_n: usize,
	bits: [u32; 193],
	stats: [u8; 193],
}

impl ExtendedSuperFrameSync {
	fn new() -> ExtendedSuperFrameSync {
		ExtendedSuperFrameSync {
			bit_n: 0,
			bits: [0u32; 193],
			stats: [0u8; 193],
		}
	}
}

impl BitSink for ExtendedSuperFrameSync {
	fn update(&mut self, bit: bool) {
		self.bits[self.bit_n] = (self.bits[self.bit_n] << 1) | (bit as u32);
		if self.bits[self.bit_n] & FRAMING_FAS_MASK == FRAMING_FAS_MATCH {
			self.stats[self.bit_n] += 1;
			if self.stats[self.bit_n] == FRAMING_FAS_MATCH_THRESHOLD {
				//esf_state.synchronize(0, ESF_FRAME_COUNT - 1);
			}
		}
	}
}

fn main() {
	let bytes = fs::read("test.u8").unwrap();
	let source = Bits::new(&bytes);
	// let file_channels = File::create("channels_x24.u8").unwrap();

	// let mut state = State::Sync(ExtendedSuperFrameSync::new());

	loop {
		let mut sync = ExtendedSuperFrameSync::new();
		for bit in source {
			sync.update(bit);
		}

		let mut demux = ExtendedSuperFrameDemux::new();
		for bit in source {
			demux.update(bit);
		}
	}
/*
	for bit in source {
		state = state.update(bit);
	}
*/
}
