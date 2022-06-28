
/// Offensively dumb UART, completely blocks on every transmitted byte.
/// We'll do better later, won't we?
/// 
pub struct Uart {
    p: u32,
}

impl Uart {
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
}
