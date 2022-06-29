use crate::peripheral::Peripheral;

pub struct USBEndpointIn {
    p: Peripheral,
}

impl USBEndpointIn {
    pub fn new(p: u32) -> Self {
        Self {
            p: Peripheral::new(p),
        }
    }

    pub fn write_fifo(&self, v: u8) {
        self.p.register_write(0, v as u32);
    }

    pub fn transmit(&self, endpoint: u8) {
        self.p.register_write(1, endpoint as u32);
    }

    pub fn reset(&self) {
        self.p.register_write(2, 1);
    }

    pub fn set_stall(&self) {
        self.p.register_write(3, 1);
    }

    pub fn clear_stall(&self) {
        self.p.register_write(3, 0);
    }

    pub fn is_idle(&self) -> bool {
        self.p.register_read(4) != 0
    }

    pub fn is_fifo_empty(&self) -> bool {
        self.p.register_read(5) == 0
    }

    pub fn is_interrupt_pending(&self) -> bool {
        self.p.register_read(6) != 0
    }

    pub fn get_pid(&self) -> u8 {
        self.p.register_read(7) as u8
    }

    pub fn set_pid(&self, pid: u8) {
        self.p.register_write(7, pid as u32);
    }
}

pub struct USBEndpointOut {
    p: Peripheral,
}

impl USBEndpointOut {
    pub fn new(p: u32) -> Self {
        Self {
            p: Peripheral::new(p),
        }
    }

    pub fn get_data(&self) -> u8 {
        self.p.register_read(0) as u8
    }

    pub fn get_data_ep(&self) -> u8 {
        self.p.register_read(1) as u8
    }

    pub fn reset(&self) {
        self.p.register_write(2, 1);
    }

    pub fn get_epno(&self) -> u8 {
        self.p.register_read(3) as u8
    }

    pub fn set_epno(&self, v: u8) {
        self.p.register_write(3, v as u32);
    }

    pub fn get_enable(&self) -> u8 {
        self.p.register_read(4) as u8
    }

    pub fn set_enable(&self, v: u8) {
        self.p.register_write(4, v as u32);
    }

    pub fn set_prime(&self, v: u8) {
        self.p.register_write(5, v as u32);
    }

    pub fn get_stall(&self) -> u8 {
        self.p.register_read(6) as u8
    }

    pub fn set_stall(&self, v: u8) {
        self.p.register_write(6, v as u32);
    }

    pub fn get_have(&self) -> u8 {
        self.p.register_read(7) as u8
    }

    pub fn set_have(&self, v: u8) {
        self.p.register_write(7, v as u32);
    }

    pub fn get_pend(&self) -> u8 {
        self.p.register_read(8) as u8
    }

    pub fn set_pend(&self, v: u8) {
        self.p.register_write(8, v as u32);
    }

    pub fn get_pid(&self) -> u8 {
        self.p.register_read(9) as u8
    }

    pub fn set_pid(&self, v: u8) {
        self.p.register_write(9, v as u32);
    }

    pub fn set_owner(&self, v: u8) {
        self.p.register_write(10, v as u32);
    }

    pub fn get_ev_status(&self) -> u32 {
        self.p.register_read(11)
    }

    pub fn get_ev_pending(&self) -> u32 {
        self.p.register_read(12)
    }

    pub fn set_ev_pending(&self, v: u32) {
        self.p.register_write(12, v);
    }

    pub fn get_ev_enable(&self) -> u32 {
        self.p.register_read(13)
    }

    pub fn set_ev_enable(&self, v: u32) {
        self.p.register_write(13, v);
    }
}
