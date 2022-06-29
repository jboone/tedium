
pub struct Peripheral {
    p: u32,
}

impl Peripheral {
    pub fn new(p: u32) -> Self {
        Self {
            p,
        }
    }

    pub fn address(&self, n: u32) -> u32 {
        self.p + n * 4
    }

    pub fn register_read(&self, n: u32) -> u32 {
        let p = self.address(n) as *const u32;
        unsafe {
            p.read_volatile()
        }
    }

    pub fn register_write(&self, n: u32, v: u32) {
        let p = self.address(n) as *mut u32;
        unsafe {
            p.write_volatile(v);
        }
    }
}
