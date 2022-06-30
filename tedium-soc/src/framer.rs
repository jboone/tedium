use xrt86vx38_pac::device::{DeviceAccess, RegisterAddress, RegisterValue, Result};

#[derive(Copy, Clone)]
pub struct Access {
    p: u32,
}

impl Access {
    pub fn new(p: u32) -> Self {
        Self {
            p,
        }
    }
}

impl DeviceAccess for Access {
    fn read(&self, address: RegisterAddress) -> Result<RegisterValue> {
        unsafe {
            let p = self.p as *const u32;
            let p = p.offset(address as isize);
            let v = p.read_volatile();
            Ok(v as RegisterValue)
        }
    }

    fn write(&self, address: RegisterAddress, value: RegisterValue) -> Result<()> {
        unsafe {
            let p = self.p as *mut u32;
            let p = p.offset(address as isize);
            p.write_volatile(value as u32);
            Ok(())
        }
    }
}

pub type Device = xrt86vx38_pac::device::Device<Access>;

pub struct FramerControl {
    p: u32
}

impl FramerControl {
    pub fn new(p: u32) -> Self {
        Self {
            p,
        }
    }

    pub fn set_reset(&self, value: bool) {
        unsafe {
            let p = self.p as *mut u32;
            let p = p.offset(0);
            p.write_volatile(value as u32);
        }
    }

    pub fn set_outputs_control(&self, value: bool) {
        unsafe {
            let p = self.p as *mut u32;
            let p = p.offset(1);
            p.write_volatile(value as u32);
        }
    }
}
