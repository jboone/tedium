use std::{sync::{Arc, Mutex}, ptr::NonNull};

use libc::c_uint;
use rusb::{ffi, Error, UsbContext, DeviceHandle};

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

// TODO: Borrowed from rusb::ffi, because it's pub(crate).
#[doc(hidden)]
pub fn from_libusb(err: i32) -> rusb::Error {
    use rusb::ffi::constants::*;

    match err {
        LIBUSB_ERROR_IO => Error::Io,
        LIBUSB_ERROR_INVALID_PARAM => Error::InvalidParam,
        LIBUSB_ERROR_ACCESS => Error::Access,
        LIBUSB_ERROR_NO_DEVICE => Error::NoDevice,
        LIBUSB_ERROR_NOT_FOUND => Error::NotFound,
        LIBUSB_ERROR_BUSY => Error::Busy,
        LIBUSB_ERROR_TIMEOUT => Error::Timeout,
        LIBUSB_ERROR_OVERFLOW => Error::Overflow,
        LIBUSB_ERROR_PIPE => Error::Pipe,
        LIBUSB_ERROR_INTERRUPTED => Error::Interrupted,
        LIBUSB_ERROR_NO_MEM => Error::NoMem,
        LIBUSB_ERROR_NOT_SUPPORTED => Error::NotSupported,
        LIBUSB_ERROR_OTHER | _ => Error::Other,
    }
}

// TODO: Keep synchronized with `gateware/descriptors_vendor.py`.
pub const INTERRUPT_BYTES_MAX: usize = 256;

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
    pub fn new_interrupt_transfer<C: UsbContext>(
        device_handle: Arc<DeviceHandle<C>>,
        endpoint: u8,
        packet_length: usize,
        timeout: c_uint,
        handler: Box<dyn TransferHandler>,
    ) -> Self {
        let buffer_length = packet_length;

        let transfer = unsafe { ffi::libusb_alloc_transfer(0) };
        let transfer = NonNull::new(transfer).expect("libusb_alloc_transfer was null");

        let mut buffer = vec![0u8; buffer_length];

        // TODO: There is certainly some leakage here. If I wasn't using these
        // structures for the duration of the process, clean-up would become
        // important. So... investigate (and fix?) at some point.

        let user_data = Box::into_raw(
            Box::new(handler)
        ).cast::<libc::c_void>();

        unsafe {
            ffi::libusb_fill_interrupt_transfer(
                transfer.as_ptr(),
                device_handle.as_raw(),
                endpoint,
                buffer.as_mut_ptr(),
                buffer.len().try_into().unwrap(),
                Self::transfer_callback,
                user_data,
                timeout
            );
        }

        Self {
            buffer,
            transfer,
        }
    }

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

pub struct CallbackWrapper<T> {
    handler: Arc<Mutex<T>>,
}

impl<T> CallbackWrapper<T> {
    pub fn new(handler: Arc<Mutex<T>>) -> Self {
        Self {
            handler,
        }
    }
}

impl<T: TransferHandler> TransferHandler for CallbackWrapper<T> {
    fn callback(&self, transfer: *mut ffi::libusb_transfer) {
        self.handler.lock().unwrap().callback(transfer);
    }
}

pub trait CallbackIn {
    fn callback_in(&mut self, transfer: *mut ffi::libusb_transfer);
}

pub struct CallbackInWrapper<T> {
    handler: Arc<Mutex<T>>,
}

impl<T> CallbackInWrapper<T> {
    pub fn new(handler: Arc<Mutex<T>>) -> Self {
        Self {
            handler,
        }
    }
}

impl<T: CallbackIn> TransferHandler for CallbackInWrapper<T> {
    fn callback(&self, transfer: *mut ffi::libusb_transfer) {
        self.handler.lock().unwrap().callback_in(transfer);
    }
}

pub trait CallbackOut {
    fn callback_out(&mut self, transfer: *mut ffi::libusb_transfer);
}

pub struct CallbackOutWrapper<T> {
    handler: Arc<Mutex<T>>,
}

impl<T> CallbackOutWrapper<T> {
    pub fn new(handler: Arc<Mutex<T>>) -> Self {
        Self {
            handler,
        }
    }
}

impl<T: CallbackOut> TransferHandler for CallbackOutWrapper<T> {
    fn callback(&self, transfer: *mut ffi::libusb_transfer) {
        self.handler.lock().unwrap().callback_out(transfer);
    }
}
