use std::{sync::Arc, ptr::NonNull};

use libc::c_uint;
use rusb::{ffi, UsbContext, DeviceHandle};

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

pub trait TransferHandler {
    fn callback(&self, transfer: *mut ffi::libusb_transfer);
}

pub struct Transfer {
    buffer: Vec<u8>,
    transfer: NonNull<ffi::libusb_transfer>,
}

#[derive(Copy, Clone)]
struct LibUsbTransferWrapper(*mut ffi::libusb_transfer);
unsafe impl Send for LibUsbTransferWrapper {}

impl Transfer {
    /// An isochronous endpoint is polled every `N` (micro)frames.
    /// Each microframe is 125 microseconds at high speed.
    /// During every polled (micro)frame, zero or more transactions may occur.
    /// Each transaction is limited to a descriptor-declared maximum length.
    /// 
    /// Interpreting this through the lens of libusb:
    /// 
    /// It appears "packet" is synonymous with "(micro)frame", in that a single
    /// libusb iso packet will contain the concatenation of however many transfers
    /// provided data in the (micro)frame.
    /// 
    /// Ideally(?), the transfer would be configured based on what the endpoint descriptor
    /// describes.
    /// 
    /// `num_iso_packets` is the number of (micro)frames this transfer embodies. If
    /// you want to be able to capture all the transfers in a microframe, ensure that
    /// the packet size is large enough to contain all the data that can be transferred
    /// in a microframe.
    /// 
    pub fn new_iso_transfer<C: UsbContext>(
        device_handle: Arc<DeviceHandle<C>>,
        endpoint: u8,
        num_iso_packets: usize,
        packet_length: usize,
        timeout: c_uint,
        // queue: FrameInQueue,
        handler: Box<dyn TransferHandler>,
    ) -> Self {
        let buffer_length = num_iso_packets * packet_length;

        let num_iso_packets = num_iso_packets.try_into().unwrap();
        let packet_length = packet_length.try_into().unwrap();

        let transfer = unsafe { ffi::libusb_alloc_transfer(num_iso_packets) };
        let transfer = NonNull::new(transfer).expect("libusb_alloc_transfer was null");

        let mut buffer = vec![0u8; buffer_length];

        // TODO: There is certainly some leakage here. If I wasn't using these
        // structures for the duration of the process, clean-up would become
        // important. So... investigate (and fix?) at some point.

        let user_data = Box::into_raw(
            Box::new(handler)
        ).cast::<libc::c_void>();

        unsafe {
            ffi::libusb_fill_iso_transfer(
                transfer.as_ptr(),
                device_handle.as_raw(),
                endpoint,
                buffer.as_mut_ptr(),
                buffer.len().try_into().unwrap(),
                num_iso_packets,
                Self::transfer_callback,
                user_data,
                timeout
            );
        }

        unsafe {
            ffi::libusb_set_iso_packet_lengths(transfer.as_ptr(), packet_length);
        }

        Self {
            buffer,
            transfer,
        }
    }

    pub fn submit(&self) {
        let result = unsafe {
            ffi::libusb_submit_transfer(self.transfer.as_ptr())
        };
		match result {
            LIBUSB_SUCCESS => {},
            e => eprintln!("libusb_submit_transfer error: {e}"),
        }
    }

    extern "system" fn transfer_callback(transfer: *mut ffi::libusb_transfer) {
        let handler = unsafe {
            let transfer = &mut *transfer;
            &mut *transfer.user_data.cast::<Box<dyn TransferHandler>>()
        };

        handler.callback(transfer);
    }
}
