
/// Offensively dumb UART, completely blocks on every transmitted byte.
/// We'll do better later, won't we?
/// 
pub struct Uart {
    p: u32,
}

const HEXCHAR: [u8; 16] = [0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66];

impl Uart {
    pub const EOL: u8 = 0x0a;
    pub const SPACE: u8 = 0x20;
    
    pub fn new(p: u32) -> Self {
        Self {
            p,
        }
    }
    
    fn register_write(&self, n: u32, v: u32) {
        unsafe {
            let p = (self.p + n * 4) as *mut u32;
            p.write_volatile(v);
        }
    }

    fn register_read(&self, n: u32) -> u32 {
        unsafe {
            let p = (self.p + n * 4) as *mut u32;
            p.read_volatile()
        }
    }

    fn tx_data(&self, v: u32) {
        self.register_write(4, v);
    }

    fn tx_rdy(&self) -> bool {
        self.register_read(5) != 0
    }

    pub fn write_char(&self, c: u8) {
        while !self.tx_rdy() {}
        self.tx_data(c as u32);
    }

    fn write_hex(&self, v: u32, digits: usize) {
        let mut x = v << ((8 - digits) * 4);
        for n in 0..digits {
            self.write_char(HEXCHAR[(x >> 28) as usize]);
            x <<= 4;
        }
    }

    pub fn write_hex_u8(&self, v: u8) {
        self.write_hex(v as u32, 2);
    }

    pub fn write_hex_u16(&self, v: u16) {
        self.write_hex(v as u32, 4);
    }

    pub fn write_hex_u32(&self, v: u32) {
        self.write_hex(v as u32, 8);
    }

    pub fn write_str(&self, s: &str) {
        for &c in s.as_bytes() {
            self.write_char(c);
        }
    }
}
