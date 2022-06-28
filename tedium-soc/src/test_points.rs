
pub struct TestPoints {
    p: u32,
    v: u32,
}

impl TestPoints {
    pub fn new(p: u32) -> Self {
        Self {
            p,
            v: 0,
        }
    }

    pub fn toggle(&mut self, n: usize) {
        self.v ^= 1 << n;
        self.set_value(self.v);
    }

    pub fn set(&mut self, n: usize) {
        self.v |= 1 << n;
        self.set_value(self.v);
    }

    pub fn clear(&mut self, n: usize) {
        self.v &= !(1 << n);
        self.set_value(self.v);
    }

    fn set_value(&self, value: u32) {
        unsafe {
            let p = self.p as *mut u32;
            let p = p.offset(0);
            p.write_volatile(value);
        }
    }
}
