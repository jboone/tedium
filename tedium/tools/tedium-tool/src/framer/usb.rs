
// TODO: Keep synchronized with `gateware/descriptors_vendor.py`.
#[derive(Copy, Clone, Debug)]
#[repr(u8)]
pub(crate) enum AlternateSetting {
    Idle = 0,
    Active = 1,
}

// TODO: Keep synchronized with `gateware/descriptors_vendor.py`.
#[derive(Copy, Clone, Debug)]
#[repr(u8)]
pub(crate) enum InterfaceNumber {
    FrameStream = 0,
    Interrupt = 1,
}

// TODO: Keep synchronized with `gateware/descriptors_vendor.py`.
#[derive(Copy, Clone, Debug)]
#[repr(u8)]
pub(crate) enum EndpointNumber {
    FrameStream = 1,
    Interrupt = 2,
}
