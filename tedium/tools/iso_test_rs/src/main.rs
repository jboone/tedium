/*
 * This file is part of Tedium.
 *
 * Copyright (C) 2020 Jared Boone <jared@sharebrained.com>
 * SPDX-License-Identifier: BSD-3-Clause
 */

use libusb;

const VENDOR_ID: u16  = 0x16d0;
const PRODUCT_ID: u16 = 0x0f3b;

fn main() {
    println!("Hello, world!");

    let context = match libusb::Context::new() {
    	Ok(context) => {
    		println!("libusb context obtained");
    		context
    	},
    	Err(e) => {
    		panic!("could not initialize libusb: {}", e);
    	},
    };

	let mut device_handle = match context.open_device_with_vid_pid(VENDOR_ID, PRODUCT_ID) {
		Some(d) => {
			println!("device opened, handle obtained");
			d
		},
		None => {
			panic!("could not open device {:04x}:{:04x}", VENDOR_ID, PRODUCT_ID);
		},
	};

	match device_handle.claim_interface(1) {
		Ok(_) => {
			println!("interface claimed");
		},
		Err(e) => {
			panic!("could not claim interface: {}", e);
		},
	}

	match device_handle.set_alternate_setting(1, 1) {
		Ok(_) => {
			println!("interface alternate setting made");
		},
		Err(e) => {
			panic!("could not change interface alternate setting: {}", e);
		},
	}

	// WHOOPS. No ISO support in libusb-rs.
}

// fn open_device(context: &mut libusb::Context, vid: u16, pid: u16) -> Option<libusb::Device, libusb::DeviceDescriptor, libusb::DeviceHandle)> {
// 	let devices = match
// }

/*
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

buf = array.array('B', (0,) * 4)

last_count = None

while True:
	n = ep_in.read(buf)
	if n == 4:
		count = struct.unpack("<I", buf)[0]

		if last_count is not None:
			diff = count - last_count
			if diff > 1:
				print('.' * (diff - 1))
		last_count = count
		# f.write(buf)
		# print(count)
	else:
		raise RuntimeError('n value unexpected')
	# ep_out.write(v)
	# print(v)
*/
