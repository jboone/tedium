use super::device::{Device, Result, RegisterAddress};

/// XRT86V38 register defaults
/// 
const DEFAULTS_CHANNEL: [(&str, &str); 46] = [
    ("N100", "11 00 __ __ __ __ __ 00  00 00 00 43 00 rr rr rr"),
    // ("N110", "rr ro 04 00 00 00 05 04  00 00 00 1c 00 40 00 __"),
    ("N110", "rr ro 04 00 rr rr 05 04  00 00 00 1c 00 40 00 __"),   // I think writing TDLBCR1 and/or RDLBCR1 might cause a spurious TxSOT interrupt?
    ("N120", "00 00 00 00 00 aa aa aa  00 80 00 aa aa 00 aa aa"),
    ("N130", "00 00 00 ff ff ff ff ff  __ __ __ ro ro ro ro ro"),
    ("N140", "00 06 00 00 00 00 00 aa  aa 00 aa aa 00 aa aa 00"),
    ("N150", "aa aa __ 00 00 00 00 aa  aa __ __ __ __ __ __ ro"),
    ("N160", "ro ro ro 00 ro ro ro ro  ro ro ro ro ro ro ro ro"),
    ("N170", "00 ro 00 00 00 00 01 __  __ __ __ __ __ __ __ __"),
    ("N180", "__ __ __ __ __ __ __ __  __ __ __ __ __ __ __ __"),
    ("N190", "__ __ __ __ __ __ __ __  __ __ __ __ __ __ __ __"),
    ("N1a0", "__ __ __ __ __ __ __ __  __ __ __ __ __ __ __ __"),
    ("N1b0", "__ __ __ __ __ __ __ __  __ __ __ __ __ __ __ __"),
    ("N1c0", "ro ro ro ro ro ro ro ro  ro ro ro ro ro ro ro ro"),
    ("N1d0", "ro ro ro ro ro ro ro ro  ro ro ro ro ro ro ro ro"),
    ("N1e0", "ro ro ro ro ro ro ro ro  ro ro ro ro ro ro ro ro"),
    ("N1f0", "__ __ __ __ __ __ __ __  __ __ __ __ __ __ __ __"),
    ("N300", "80 80 80 80 80 80 80 80  80 80 80 80 80 80 80 80"),
    ("N310", "80 80 80 80 80 80 80 80  80 80 80 80 80 80 80 80"),
    ("N320", "17 17 17 17 17 17 17 17  17 17 17 17 17 17 17 17"),
    ("N330", "17 17 17 17 17 17 17 17  17 17 17 17 17 17 17 17"),
    ("N340", "01 d0 d0 d0 d0 d0 d0 d0  d0 d0 d0 d0 d0 d0 d0 d0"),
    ("N350", "b3 d0 d0 d0 d0 d0 d0 d0  d0 d0 d0 d0 d0 d0 d0 d0"),
    ("N360", "80 80 80 80 80 80 80 80  80 80 80 80 80 80 80 80"),
    ("N370", "80 80 80 80 80 80 80 80  80 80 80 80 80 80 80 80"),
    ("N380", "ff ff ff ff ff ff ff ff  ff ff ff ff ff ff ff ff"),
    ("N390", "ff ff ff ff ff ff ff ff  ff ff ff ff ff ff ff ff"),
    ("N3a0", "00 00 00 00 00 00 00 00  00 00 00 00 00 00 00 00"),
    ("N3b0", "00 00 00 00 00 00 00 00  00 00 00 00 00 00 00 00"),
    ("N3c0", "00 00 00 00 00 00 00 00  00 00 00 00 00 00 00 00"),
    ("N3d0", "00 00 00 00 00 00 00 00  00 00 00 00 00 00 00 00"),
    ("N3e0", "__ __ __ __ __ __ __ __  __ __ __ __ __ __ __ __"),
    ("N3f0", "__ __ __ __ __ __ __ __  __ __ __ __ __ __ __ __"),
    ("N500", "ro ro ro ro ro ro ro ro  ro ro ro ro ro ro ro ro"),
    ("N510", "ro ro ro ro ro ro ro ro  ro ro ro ro ro ro ro ro"),
    ("N900", "rr rr rr rr rr rr rr __  __ rr rr rr rr rr rr rr"),
    ("N910", "rr rr __ __ __ __ __ __  __ __ __ __ rr __ __ __"),
    ("N920", "__ __ __ __ __ __ __ __  __ __ __ __ rr __ __ __"),
    ("Nb00", "ro 00 rr 00 rr 00 rr 00  rr 00 rr 00 __ __ rr 00"),
    ("Nb10", "rr 00 rr 00 rr 00 rr 00  rr 00 rr 00 rr 00 rr 00"),
    ("Nb20", "rr 00 rr 00 rr 00 rr 00  rr 00 __ __ __ __ __ __"),
    ("Nb30", "__ __ __ __ __ __ __ __  __ __ __ __ __ __ __ __"),
    ("Nb40", "rr 00 __ __ __ __ __ __  __ __ __ __ __ __ __ __"),
    ("Nb50", "__ __ __ __ __ __ __ __  __ __ __ __ __ __ __ __"),
    ("Nb60", "__ __ __ __ __ __ __ __  __ __ __ __ __ __ __ __"),
    ("Nb70", "rr 00 __ __ rr 00 __ __  __ __ __ __ __ __ __ __"),
    ("0fN0", "00 00 00 00 00 ro rr ro  00 00 00 00 00 00 00 00"),
];

// TODO: Perform global configuration?
// const DEFAULTS_GLOBAL: [(&str, &str); 3] = [
//     ("0fe0", "00 00 00 __ 00 __ __ __  __ 01 rr __ __ __ __ __"),
//     ("0102", "f0"),
//     ("4102", "00"),
// ];

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum DefaultsMode {
    Write,
    Check
}

/// Apply XRT86V38 register defaults to a device through its "uP" interface,
/// or check device register contents against defaults.
/// 
pub fn register_defaults(device: &Device, configure_mode: DefaultsMode) -> Result<()> {
	device.framer_interface_control(false)?;

    // Notes:
    // 
    // On power-up:
    // 0xN112 RIFR[2] = 1 but datasheet says 0. Doesn't seem to matter unless another bit (defaut 0) is set to 1.
    // 0xN340 TSCR0[7:4] (transmit signaling bits A-D) = 0b0000, but datasheet says "N".
    // 0x4102 GPIOCR1[7:4] = 0b1111 (outputs), but datasheet says 0b0000 (inputs). The docs for 0x0102 GPIOCR0 are correct though.

    // TODO: Perform global configuration?

    match configure_mode {
        DefaultsMode::Write => { println!("writing default configuration") },
        DefaultsMode::Check => { println!("comparing current configuration with register defaults")},
    }

    if configure_mode == DefaultsMode::Write {
        // At power up, I read: 0fe0: 00 00 00 00 00 00 00 ff ff 01 00 00 ff 00 00 00
        device.register_write(0x0fe0, 0b0000_0000)?;
        device.register_write(0x0fe1, 0b0000_0000)?;
        device.register_write(0x0fe2, 0b0000_0000)?;
        device.register_write(0x0fe4, 0b0000_0000)?;
        device.register_write(0x0fe9, 0b0000_0001)?;
    }

	for channel in device.channels() {
        let channel_index_str = format!("{:1}", channel.index());
		for (first_address, values) in DEFAULTS_CHANNEL {
            let first_address = first_address.replace('N', &channel_index_str);
			let first_address = usize::from_str_radix(&first_address, 16).unwrap();

			let values = values.split_whitespace();

			// for (address, value) in enumerate(values, start=first_address):
			for (address, value) in values.enumerate() {
                let address = first_address + address;
                assert!(address < 0x10000);
                let address = address as RegisterAddress;
                
                // println!("{address:04x} {value}");

                match value {
                    "__" => {},
                    "ro" => {},
                    "rr" => {
		    			device.register_read(address)?;
                    },
                    s => {
                        let value = usize::from_str_radix(s, 16).unwrap();
                        assert!(value < 0x100);
                        let value = value as u8;

                        match configure_mode {
                            DefaultsMode::Write => {
				        		device.register_write(address, value)?;
                            },
                            DefaultsMode::Check => {
                                let read_value = device.register_read(address)?;
                                if read_value != value {
                                    println!("{:04x} {:02x} != {:02x}", address, read_value, value);
                                }
                            },
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
