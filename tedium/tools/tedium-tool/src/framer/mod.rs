use crate::detector::DetectionEvent;

use self::audio::TimeslotAddress;

pub mod audio;
pub mod device;
pub mod dump;
pub mod interrupt;
pub mod register;
pub mod test;
mod usb;

#[derive(Copy, Clone, Debug)]
pub enum FramerEvent {
    Interrupt { data: [u8; usb::INTERRUPT_BYTES_MAX], length: usize },
    Digit(TimeslotAddress, DetectionEvent),
}
