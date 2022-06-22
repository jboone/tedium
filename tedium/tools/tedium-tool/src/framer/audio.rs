use std::cmp::{min, max, self};
use std::collections::{VecDeque, HashMap};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::mem::size_of;
use std::ptr::NonNull;
use std::slice;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::thread;

use crate::codec::ulaw;
use crate::detector::{dtmf, Detector};
use crate::framer::device::open_device;
use crate::generator::ToneGenerator;
use crate::generator::dual_tone::DualToneGenerator;

use audio_thread_priority::promote_current_thread_to_real_time;
use bytemuck::{Pod, Zeroable, bytes_of};
use crossbeam::channel::{unbounded, Sender};
use libc::c_uint;
use ringbuf::{RingBuffer, Consumer, Producer};
use rusb::constants::LIBUSB_TRANSFER_COMPLETED;
use rusb::ffi::{libusb_set_iso_packet_lengths, libusb_get_iso_packet_buffer};
use rusb::{ffi, UsbContext, DeviceHandle};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PumpError {
    #[error("libusb")]
    LibUsb(i32),
    #[error(transparent)]
    Rusb(#[from] rusb::Error),
}

pub type Sample = u8;

const CHANNELS: usize = 8;
const TIMESLOTS_PER_CHANNEL: usize = 24;
const TIMESLOTS_PER_FRAME: usize = TIMESLOTS_PER_CHANNEL * CHANNELS;

const FRAME_OUT_LENGTH: usize = TIMESLOTS_PER_FRAME;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
struct TimeslotAddress {
    channel: usize,
    timeslot: usize,
}

impl TimeslotAddress {
    fn new(channel: usize, timeslot: usize) -> Self {
        Self {
            channel,
            timeslot,
        }
    }
}

#[derive(Clone, Debug)]
enum AudioMessage {
    Fragment(Vec<FrameIn>),
}

#[derive(Copy, Clone, Debug)]
#[repr(packed)]
struct FrameIn {
    framer_frame_count: u32,
    usb_in_fifo_level: u16,
    usb_out_fifo_level: u16,
    fifo_rx_overflow_count: u16,
    fifo_tx_underflow_count: u16,
    timeslot: [[Sample; CHANNELS]; TIMESLOTS_PER_CHANNEL],
    // fifo_tx_overflow_count: u32,
    // fifo_tx_underflow_count: u32,
    // framer_bit_count: u32,
    // usb_in_byte_count: u32,
    // usb_out_byte_count: u32,
    // usb_sof_count: u32,
    // flags: u8,
    // _padding: [u8; 27],
}

const FRAME_IN_LENGTH: usize = size_of::<FrameIn>();

unsafe impl Zeroable for FrameIn {}
unsafe impl Pod for FrameIn {}

impl FrameIn {
    fn timeslot(&self, address: &TimeslotAddress) -> Sample {
        self.timeslot[address.timeslot][address.channel]
    }
}

impl Default for FrameIn {
    fn default() -> Self {
        Self {
            framer_frame_count: 0,
            usb_in_fifo_level: 0,
            usb_out_fifo_level: 0,
            fifo_rx_overflow_count: 0,
            fifo_tx_underflow_count: 0,
            timeslot: [[0xff; CHANNELS]; TIMESLOTS_PER_CHANNEL],
            // fifo_tx_overflow_count: 0,
            // fifo_tx_underflow_count: 0,
            // framer_bit_count: 0,
            // usb_in_byte_count: 0,
            // usb_out_byte_count: 0,
            // usb_sof_count: 0,
            // flags: 0,
            // _padding: [0; 27],
        }
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(packed)]
struct FrameOut {
    timeslot: [[Sample; CHANNELS]; TIMESLOTS_PER_CHANNEL],
}

unsafe impl Zeroable for FrameOut {}
unsafe impl Pod for FrameOut {}

impl FrameOut {
    fn timeslot_mut(&mut self, address: &TimeslotAddress) -> &mut Sample {
        &mut self.timeslot[address.timeslot][address.channel]
    }
}

impl Default for FrameOut {
    fn default() -> Self {
        Self {
            timeslot: [[0xff; CHANNELS]; TIMESLOTS_PER_CHANNEL],
        }
    }
}

trait IsochronousTransferHandler {
    fn callback(&self, transfer: *mut ffi::libusb_transfer);
}

struct IsochronousTransfer {
    buffer: Vec<u8>,
    transfer: NonNull<ffi::libusb_transfer>,
}

#[derive(Copy, Clone)]
struct LibUsbTransferWrapper(*mut ffi::libusb_transfer);
unsafe impl Send for LibUsbTransferWrapper {}

type FrameInQueue = Arc<Mutex<VecDeque<FrameIn>>>;
// type TransactionsOutQueue = Arc<Mutex<VecDeque<LibUsbTransferWrapper>>>;
// type FrameInQueue = Arc<Mutex<VecDeque<FrameOut>>>;

impl IsochronousTransfer {
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
    fn new<C: UsbContext>(
        device_handle: Arc<DeviceHandle<C>>,
        endpoint: u8,
        num_iso_packets: usize,
        packet_length: usize,
        timeout: c_uint,
        // queue: FrameInQueue,
        handler: Box<dyn IsochronousTransferHandler>,
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
                Self::iso_transfer_callback,
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

    fn submit(&self) {
        let result = unsafe {
            ffi::libusb_submit_transfer(self.transfer.as_ptr())
        };
		match result {
            LIBUSB_SUCCESS => {},
            e => eprintln!("libusb_submit_transfer error: {e}"),
        }
    }

    extern "system" fn iso_transfer_callback(transfer: *mut ffi::libusb_transfer) {
        let handler = unsafe {
            let transfer = &mut *transfer;
            &mut *transfer.user_data.cast::<Box<dyn IsochronousTransferHandler>>()
        };

        handler.callback(transfer);
    }
}

///////////////////////////////////////////////////////////////////////

struct AudioFrameInCallback {
    handler: Arc<Mutex<AudioFrameInHandler>>,
}

impl AudioFrameInCallback {
    fn new(handler: Arc<Mutex<AudioFrameInHandler>>) -> Self {
        Self {
            handler,
        }
    }
}

impl IsochronousTransferHandler for AudioFrameInCallback {
    fn callback(&self, transfer: *mut ffi::libusb_transfer) {
        self.handler.lock().unwrap().handle(transfer);
    }
}

struct AudioFrameInHandler {
    queue_in: FrameInQueue,
    queue_log: FrameInQueue,
    transfer_in_count: u64,
    frame_in_count: u64,
}

impl AudioFrameInHandler {
    fn new(queue_in: FrameInQueue, queue_log: FrameInQueue) -> Self {
        Self {
            queue_in,
            queue_log,
            transfer_in_count: 0,
            frame_in_count: 0,
        }
    }

    fn handle_frame_in(&mut self, frame: FrameIn) {
        self.queue_in.lock().unwrap().push_back(frame);
        self.queue_log.lock().unwrap().push_back(frame);

        self.frame_in_count += 1;
    }

    fn handle(&mut self, transfer: *mut ffi::libusb_transfer) {
        use rusb::ffi::constants::*;
    
        self.transfer_in_count += 1;

        let transfer_status = unsafe { (*transfer).status };
        if transfer_status == LIBUSB_TRANSFER_COMPLETED {
            let num_iso_packets = unsafe { (*transfer).num_iso_packets }.try_into().unwrap();
            for i in 0..num_iso_packets {
                let packet = unsafe { (*transfer).iso_packet_desc.get_unchecked(i) };
                if packet.status == LIBUSB_TRANSFER_COMPLETED {
                    let packet_actual_length: usize = packet.actual_length.try_into().unwrap();
                    if packet_actual_length > 0 {
                        if packet_actual_length % FRAME_IN_LENGTH == 0 {
                            let b = unsafe { ffi::libusb_get_iso_packet_buffer_simple(transfer, i.try_into().unwrap()) };
                            if let Some(b) = NonNull::new(b) {
                                let buffer = unsafe { slice::from_raw_parts_mut(b.as_ptr(), packet_actual_length) };
                                for slice in buffer.chunks_exact(FRAME_IN_LENGTH) {
                                    let mut frame_in = FrameIn::default();
                                    let frame_in_bytes = bytemuck::bytes_of_mut(&mut frame_in);
                                    frame_in_bytes.copy_from_slice(slice);

                                    self.handle_frame_in(frame_in);
                                }
                            } else {
                                eprintln!("IN: packet[{i:2}] null pointer");
                            }
                        } else {
                            eprintln!("IN: packet[{i:2}] actual_length={} not a multiple of frame length", packet_actual_length);
                        }
                    // } else {
                    //     eprintln!("IN: packet[{i:2}] actual_length={}", packet_actual_length);
                    }
                } else {
                    eprintln!("IN: packet[{i:2}] status {}", packet.status);
                }
            }
        } else {
            eprintln!("IN: transfer status: {}", transfer_status);
        }

        let result = unsafe {
            ffi::libusb_submit_transfer(transfer)
        };
        match result {
            LIBUSB_SUCCESS => {},
            e => eprintln!("IN: libusb_submit_transfer error: {e}"),
        }
    }
}

///////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]

enum ToneSource {
    DialTonePrecise,
    Ringback,
}

enum Patch {
    Idle,
    Input(TimeslotAddress),
    Tone(ToneSource),
}

struct AudioFrameOutCallback {
    handler: Arc<Mutex<AudioFrameOutHandler>>,
}

impl AudioFrameOutCallback {
    fn new(handler: Arc<Mutex<AudioFrameOutHandler>>) -> Self {
        Self {
            handler,
        }
    }
}

impl IsochronousTransferHandler for AudioFrameOutCallback {
    fn callback(&self, transfer: *mut ffi::libusb_transfer) {
        self.handler.lock().unwrap().handle(transfer);
    }
}

struct Patching {
    map: [[Patch; CHANNELS]; TIMESLOTS_PER_CHANNEL],
}

impl Patching {
    fn timeslot(&self, address: &TimeslotAddress) -> &Patch {
        &self.map[address.timeslot][address.channel]
    }
}

impl Default for Patching {
    fn default() -> Self {
        use Patch::*;

        Self {
            map: [
                [Idle, Idle, Idle, Idle, Idle, Idle, Idle, Idle,],  // Timeslot 00
                [Idle, Idle, Idle, Idle, Idle, Idle, Idle, Idle,],
                [Idle, Idle, Idle, Idle, Idle, Idle, Idle, Idle,],
                [Idle, Idle, Idle, Idle, Idle, Idle, Idle, Idle,],
                [Idle, Idle, Idle, Idle, Idle, Idle, Idle, Idle,],
                [Idle, Idle, Idle, Idle, Idle, Idle, Idle, Idle,],
                [Idle, Idle, Idle, Idle, Idle, Idle, Idle, Idle,],
                [Idle, Idle, Idle, Idle, Idle, Idle, Idle, Idle,],
                [Idle, Idle, Idle, Idle, Idle, Idle, Idle, Idle,],  // Timeslot 08
                [Idle, Idle, Idle, Idle, Idle, Idle, Idle, Idle,],
                [Idle, Idle, Idle, Idle, Idle, Idle, Idle, Idle,],
                [Idle, Idle, Idle, Idle, Idle, Idle, Idle, Idle,],
                [Idle, Idle, Idle, Idle, Idle, Idle, Idle, Idle,],
                [Idle, Idle, Idle, Idle, Idle, Idle, Idle, Idle,],
                [Idle, Idle, Idle, Idle, Idle, Idle, Idle, Idle,],
                [Idle, Idle, Idle, Idle, Idle, Idle, Idle, Idle,],
                [Idle, Idle, Idle, Idle, Idle, Idle, Idle, Idle,],  // Timeslot 16
                [Idle, Idle, Idle, Idle, Idle, Idle, Idle, Idle,],
                [Idle, Idle, Idle, Idle, Idle, Idle, Idle, Idle,],
                [Idle, Idle, Idle, Idle, Idle, Idle, Idle, Idle,],
                [Idle, Idle, Idle, Idle, Idle, Idle, Idle, Idle,],
                [Idle, Idle, Idle, Idle, Idle, Idle, Idle, Idle,],
                [Idle, Idle, Idle, Idle, Idle, Idle, Idle, Idle,],
                [Idle, Idle, Idle, Idle, Idle, Idle, Idle, Idle,],  // Timeslot 23
            ],
        }
    }
}

struct AudioFrameOutHandler {
    queue_frames: FrameInQueue,
    transfer_count: usize,

    // TX FIFO level min/max during the last transfer.
    framer_fifo_tx_level_minmax: Option<(u16, u16)>,

    patching: Patching,
    tone_plant: HashMap<ToneSource, Box<dyn ToneGenerator>>,
    detectors: HashMap<TimeslotAddress, Box<dyn Detector>>,
}

impl AudioFrameOutHandler {

    fn new(queue_frames: FrameInQueue) -> Self {
        use ToneSource::*;

        let mut tone_plant: HashMap<ToneSource, Box<dyn ToneGenerator>> = HashMap::new();
        tone_plant.insert(ToneSource::DialTonePrecise, Box::new(DualToneGenerator::new(350.0, 440.0)));
        tone_plant.insert(ToneSource::Ringback, Box::new(DualToneGenerator::new(440.0, 480.0)));

        let mut detectors: HashMap<TimeslotAddress, Box<dyn Detector>> = HashMap::new();
        detectors.insert(TimeslotAddress::new(1, 0), Box::new(dtmf::Detector::new()));

        Self {
            queue_frames,
            transfer_count: 0,
            framer_fifo_tx_level_minmax: None,
            patching: Patching::default(),
            tone_plant,
            detectors,
        }
    }

    fn process_frame(&mut self, frame_in: &FrameIn) -> FrameOut {
        // Update generator outputs.
        for generator in self.tone_plant.values_mut() {
            generator.advance();
        }

        // Update detectors with new input samples.
        for (address, detector) in &mut self.detectors {
            let sample_ulaw = frame_in.timeslot(&address);
            let sample_linear = ulaw::decode(sample_ulaw);
            if let Some(output) = detector.advance(sample_linear) {
                eprintln!("detect: {output:3.0?}");
            }
        }

        // Compute output samples.
        let mut frame_out = FrameOut::default();
        for out_channel in 0..CHANNELS {
            for out_timeslot in 0..TIMESLOTS_PER_CHANNEL {
                let timeslot_address = TimeslotAddress::new(out_channel, out_timeslot);
                *frame_out.timeslot_mut(&timeslot_address) =
                    match &self.patching.timeslot(&timeslot_address) {
                        Patch::Idle => 0xff,
                        Patch::Input(address) => {
                            frame_in.timeslot(&address)
                        },
                        Patch::Tone(source) => {
                            let output = if let Some(generator) = self.tone_plant.get(source) {
                                generator.output()
                            } else {
                                0.0
                            };
                            ulaw::encode(output)
                        },
                    };
            }
        }

        frame_out
    }

    fn handle(&mut self, transfer: *mut ffi::libusb_transfer) {
        // Assumptions: IN and OUT frame rates *should* be precisely the same
        // as they're determined by the framer clock.

        // And the number of packets in this transfer.
        let packets_in_transfer: usize = unsafe { (*transfer).num_iso_packets.try_into().unwrap() };

        // Single contiguous buffer allocated to transfer.
        let buffer = unsafe { slice::from_raw_parts_mut((*transfer).buffer, (*transfer).length.try_into().unwrap()) };

        // Decide whether the first packet will be long (two frames) or short (zero frames)
        // in order to adjust the TX FIFO level.
        let enable_adjustment = (self.transfer_count & 0x7f) == 0;
        let first_packet_frame_count: usize = if enable_adjustment {
            const TX_FIFO_LOWER_THRESHOLD: u16 = 3000;
            const TX_FIFO_UPPER_THRESHOLD: u16 = 4000;

            // NOTE: If we had no frames available during the last transfer, the TX FIFO levels
            // will be set to MAX/MIN values. I suspect this will cut me at some point down the road...

            if let Some((min, max)) = self.framer_fifo_tx_level_minmax {
                if min > TX_FIFO_LOWER_THRESHOLD && max > TX_FIFO_UPPER_THRESHOLD {
                    0  // Send one less frame.
                } else if min < TX_FIFO_LOWER_THRESHOLD && max < TX_FIFO_UPPER_THRESHOLD {
                    2   // Send one more frame.
                } else {
                    1   // No adjustment.
                }
            } else {
                1   // No stats, so no adjustment.
            }
        } else {
            1   // Not the time to make an adjustment.
        };

        // Configure the first packet length.
        let packet = unsafe { (*transfer).iso_packet_desc.get_unchecked_mut(0) };
        packet.length = (first_packet_frame_count * FRAME_OUT_LENGTH).try_into().unwrap();

        // Compute the lengths of the entire transfer.
        let frames_in_transfer = first_packet_frame_count + (packets_in_transfer - 1);
        // let bytes_in_transfer = frames_in_transfer * FRAME_OUT_LENGTH;

        // Set remaining packet lengths to the nominal condition, one frame per packet.
        for i in 1..packets_in_transfer {
            let packet = unsafe { (*transfer).iso_packet_desc.get_unchecked_mut(i) };
            packet.length = FRAME_OUT_LENGTH.try_into().unwrap();
            // TODO: packet.actual_length doesn't matter for USB OUT transfers, right?
        }

        // Reset stats for collection during this transfer.
        self.framer_fifo_tx_level_minmax = None;

        // Copy the required number of frames into the transfer buffer.
        for slice in buffer.chunks_exact_mut(FRAME_OUT_LENGTH).take(frames_in_transfer) {
            let frame_in = self.queue_frames.lock().unwrap().pop_front();
            if let Some(frame_in) = frame_in {
                // Update TX FIFO statistics for this transfer.
                let framer_fifo_tx_level = frame_in.usb_out_fifo_level;

                self.framer_fifo_tx_level_minmax = Some(
                    if let Some((min, max)) = self.framer_fifo_tx_level_minmax {
                        let new_min = cmp::min(min, framer_fifo_tx_level);
                        let new_max = cmp::max(max, framer_fifo_tx_level);
                        (new_min, new_max)
                    } else {
                        (framer_fifo_tx_level, framer_fifo_tx_level)
                    }
                );

                let frame_out = self.process_frame(&frame_in);

                let frame_out_bytes = bytemuck::bytes_of(&frame_out);
                slice.copy_from_slice(frame_out_bytes);
            } else {
                // No frame data is available, send a single idle frame in this packet.
                slice.fill(0xff);
            }
        }

        // TODO: Is this necessary for isochronous transfers?
        // transfer.actual_length = bytes_in_transfer;

        let result = unsafe {
            ffi::libusb_submit_transfer(transfer)
        };
        match result {
            ffi::constants::LIBUSB_SUCCESS => {},
            e => eprintln!("OUT: libusb_submit_transfer error: {e}"),
        }

        self.transfer_count += 1;
    }
}

///////////////////////////////////////////////////////////////////////

// TODO: Keep synchronized with `gateware/descriptors_vendor.py`.
#[derive(Copy, Clone, Debug)]
#[repr(u8)]
enum AlternateSetting {
    Idle = 0,
    Active = 1,
}

// TODO: Keep synchronized with `gateware/descriptors_vendor.py`.
#[derive(Copy, Clone, Debug)]
#[repr(u8)]
enum InterfaceNumber {
    FrameStream = 0,
    Interrupt = 1,
}

// TODO: Keep synchronized with `gateware/descriptors_vendor.py`.
#[derive(Copy, Clone, Debug)]
#[repr(u8)]
enum EndpointNumber {
    FrameStream = 1,
    Interrupt = 2,
}

pub fn pump3() -> Result<(), PumpError> {
    let mut context = rusb::Context::new()?;

    let mut device = open_device(&mut context)?;

    use rusb::constants::*;
    
    device.claim_interface(InterfaceNumber::FrameStream as u8)?;
    device.set_alternate_setting(InterfaceNumber::FrameStream as u8, AlternateSetting::Active as u8)?;

    // TODO: Grab endpoint addresses from descriptors, instead of hard-coding them?

    let device = Arc::new(device);

    let queue_frames_in: FrameInQueue = Arc::new(Mutex::new(VecDeque::with_capacity(100)));
    let queue_frames_log: FrameInQueue = Arc::new(Mutex::new(VecDeque::with_capacity(100)));

    let device_handle = device;

    const NUM_ISO_PACKETS: usize = 8;   // For each USB transfer request (kernel URB?), get one millisecond worth of frames.

    let mut transfers_in: Vec<IsochronousTransfer> = Vec::new();
    let mut transfers_out: Vec<IsochronousTransfer> = Vec::new();

    let handler_in = Arc::new(Mutex::new(AudioFrameInHandler::new(
        queue_frames_in.clone(),
        queue_frames_log.clone(),
    )));

    let handler_out = Arc::new(Mutex::new(AudioFrameOutHandler::new(
        queue_frames_in.clone(),
    )));

    for _ in 0..8 {
        let transfer_in = IsochronousTransfer::new(
            device_handle.clone(),
            LIBUSB_ENDPOINT_IN | EndpointNumber::FrameStream as u8,
            NUM_ISO_PACKETS,
            FRAME_IN_LENGTH * 2,    // Two transfers (audio frames) per USB microframe.
            0,
            Box::new(AudioFrameInCallback::new(handler_in.clone())),
        );

        transfer_in.submit();
        transfers_in.push(transfer_in);

        let transfer_out = IsochronousTransfer::new(
            device_handle.clone(),
            LIBUSB_ENDPOINT_OUT | EndpointNumber::FrameStream as u8,
            NUM_ISO_PACKETS,
            FRAME_OUT_LENGTH * 2,
            0,
            Box::new(AudioFrameOutCallback::new(handler_out.clone())),
        );

        transfer_out.submit();
        transfers_out.push(transfer_out);
    }

    thread::Builder::new()
        .name("log".to_string())
        .spawn({
            move || {
                let file_out = File::create("/tmp/blah_frames.bin").unwrap();
                let mut file_out = BufWriter::new(file_out);

                let mut count = 0u64;
                let mut framer_count_last = None;

                let mut rx_fifo_count_min = u16::MAX;
                let mut rx_fifo_count_max = u16::MIN;

                let mut tx_fifo_count_min = u16::MAX;
                let mut tx_fifo_count_max = u16::MIN;
            
                loop {
                    let popped = queue_frames_log.lock().expect("mutex poisoned").pop_front();
                    if let Some(frame) = popped {
                        let frame_in_bytes = bytemuck::bytes_of(&frame);
                        file_out.write_all(frame_in_bytes).expect("file out: write");

                        let rx_fifo_count = frame.usb_in_fifo_level;
                        let tx_fifo_count = frame.usb_out_fifo_level;

                        rx_fifo_count_min = min(rx_fifo_count_min, rx_fifo_count);
                        rx_fifo_count_max = max(rx_fifo_count_max, rx_fifo_count);
                        tx_fifo_count_min = min(tx_fifo_count_min, tx_fifo_count);
                        tx_fifo_count_max = max(tx_fifo_count_max, tx_fifo_count);

                        count += 1;

                        if count % 8000 == 0 {
                            let framer_count = frame.framer_frame_count;
                            let framer_count_diff = if let Some(framer_count_last) = framer_count_last {
                                framer_count.wrapping_sub(framer_count_last)
                            } else {
                                0
                            };
                            framer_count_last = Some(framer_count);
                            
                            let (queue_in_len, queue_in_capacity) = {
                                let d = queue_frames_in.lock().expect("mutex poisoned");
                                (d.len(), d.capacity())
                            };
                            
                            let (queue_log_len, queue_log_capacity) = {
                                let d = queue_frames_log.lock().expect("mutex poisoned");
                                (d.len(), d.capacity())
                            };

                            eprintln!("framer(rx={rx_fifo_count_min:4}..{rx_fifo_count_max:4} tx={tx_fifo_count_min:4}..{tx_fifo_count_max:4}, count:{framer_count_diff:+5}={framer_count:5}) count={count:9} queue_in={queue_in_len:4}/{queue_in_capacity:4} queue_log={queue_log_len:4}/{queue_log_capacity:4}");

                            rx_fifo_count_min = u16::MAX;
                            rx_fifo_count_max = u16::MIN;
                            tx_fifo_count_min = u16::MAX;
                            tx_fifo_count_max = u16::MIN;
                        }
                    } else {
                        // TODO: This seems... less than ideal.
                        thread::sleep(Duration::from_millis(10));
                    }
                }
            }
        }).expect("log: thread::spawn()");

    promote_current_thread_to_real_time(8 * 4, 8000).unwrap();

    loop {
        let result = unsafe {
            ffi::libusb_handle_events(context.as_raw())
        };
        if result != 0 {
            eprintln!("error: libusb_handle_events: {:?}", result);
            return Err(PumpError::LibUsb(result));
        }
    }

    Ok(())
}

pub fn pump_loopback() -> Result<(), PumpError> {
    let mut context = rusb::Context::new()?;

    let mut device = open_device(&mut context)?;

    use rusb::constants::*;
    
    device.claim_interface(InterfaceNumber::FrameStream as u8)?;
    device.set_alternate_setting(InterfaceNumber::FrameStream as u8, AlternateSetting::Active as u8)?;

    // TODO: Grab endpoint addresses from descriptors, instead of hard-coding them?

    let device = Arc::new(device);

    let device_handle = device;

    const NUM_ISO_PACKETS: usize = 8;   // For each USB transfer request (kernel URB?), get one millisecond worth of frames.

    let mut transfers_in: Vec<IsochronousTransfer> = Vec::new();
    let mut transfers_out: Vec<IsochronousTransfer> = Vec::new();

    let handler = Arc::new(Mutex::new(LoopbackFrameHandler::new()));

    for _ in 0..8 {
        let transfer_in = IsochronousTransfer::new(
            device_handle.clone(),
            LIBUSB_ENDPOINT_IN | EndpointNumber::FrameStream as u8,
            NUM_ISO_PACKETS,
            512,
            0,
            Box::new(CallbackInWrapper::new(handler.clone())),
        );

        transfer_in.submit();
        transfers_in.push(transfer_in);

        let transfer_out = IsochronousTransfer::new(
            device_handle.clone(),
            LIBUSB_ENDPOINT_OUT | EndpointNumber::FrameStream as u8,
            NUM_ISO_PACKETS,
            512,
            0,
            Box::new(CallbackOutWrapper::new(handler.clone())),
        );

        transfer_out.submit();
        transfers_out.push(transfer_out);
    }

    promote_current_thread_to_real_time(8, 8000).unwrap();

    loop {
        let result = unsafe {
            ffi::libusb_handle_events(context.as_raw())
        };
        if result != 0 {
            eprintln!("error: libusb_handle_events: {:?}", result);
            return Err(PumpError::LibUsb(result));
        }
    }

    // Ok(())
}

trait CallbackIn {
    fn callback_in(&mut self, transfer: *mut ffi::libusb_transfer);
}

struct CallbackInWrapper<T> {
    handler: Arc<Mutex<T>>,
}

impl<T> CallbackInWrapper<T> {
    fn new(handler: Arc<Mutex<T>>) -> Self {
        Self {
            handler,
        }
    }
}

impl<T: CallbackIn> IsochronousTransferHandler for CallbackInWrapper<T> {
    fn callback(&self, transfer: *mut ffi::libusb_transfer) {
        self.handler.lock().unwrap().callback_in(transfer);
    }
}

trait CallbackOut {
    fn callback_out(&mut self, transfer: *mut ffi::libusb_transfer);
}

struct CallbackOutWrapper<T> {
    handler: Arc<Mutex<T>>,
}

impl<T> CallbackOutWrapper<T> {
    fn new(handler: Arc<Mutex<T>>) -> Self {
        Self {
            handler,
        }
    }
}

impl<T: CallbackOut> IsochronousTransferHandler for CallbackOutWrapper<T> {
    fn callback(&self, transfer: *mut ffi::libusb_transfer) {
        self.handler.lock().unwrap().callback_out(transfer);
    }
}

#[derive(Copy, Clone, Debug)]
struct Frame {
    timeslot: [[Sample; CHANNELS]; TIMESLOTS_PER_CHANNEL],
}

struct InternalFrame {
    frame: Frame,
    frame_count: u32,
}

struct LoopbackFrameHandler {
    frames_in: Producer<InternalFrame>,
    frames_out: Consumer<InternalFrame>,
    sof_count_next: u32,
    frame_count_next: u32,
    tx_fifo_level_min: u8,
}

impl LoopbackFrameHandler {
    fn new() -> Self {
        let (producer, consumer) = RingBuffer::new(40).split();
        Self {
            frames_in: producer,    // 40 frames == 5 milliseconds.
            frames_out: consumer,
            sof_count_next: 0,
            frame_count_next: 0,
            tx_fifo_level_min: 0,
        }
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(packed)]
struct RxFrameReport {
    frame_count: u32,
}

#[derive(Copy, Clone, Debug)]
#[repr(packed)]
struct RxFrame {
    frame: Frame,
    report: RxFrameReport,
}

unsafe impl Zeroable for RxFrame {}
unsafe impl Pod for RxFrame {}

#[derive(Copy, Clone, Debug)]
#[repr(packed)]
struct RxUSBReport {
    sof_count: u32,
    fifo_rx_level: u8,
    fifo_tx_level: u8,
    fifo_rx_underflow_count: u16,
    fifo_tx_overflow_count: u16,
    sequence_count: u8,
}

unsafe impl Zeroable for RxUSBReport {}
unsafe impl Pod for RxUSBReport {}

#[derive(Copy, Clone, Debug)]
#[repr(packed)]
struct TxUSBReport {
    frame_count: u32,
}

unsafe impl Zeroable for TxUSBReport {}
unsafe impl Pod for TxUSBReport {}

#[derive(Copy, Clone, Debug)]
#[repr(packed)]
struct TxFrameReport {
    frame_count: u32,
}

#[derive(Copy, Clone, Debug)]
#[repr(packed)]
struct TxFrame {
    report: TxFrameReport,
    frame: Frame,
}

unsafe impl Zeroable for TxFrame {}
unsafe impl Pod for TxFrame {}

impl CallbackIn for LoopbackFrameHandler {
    fn callback_in(&mut self, transfer: *mut ffi::libusb_transfer) {
        let transfer_status = unsafe { (*transfer).status };
        if transfer_status != LIBUSB_TRANSFER_COMPLETED {
            eprintln!("IN: transfer.status = {transfer_status}");
        }

        let num_iso_packets = unsafe { (*transfer).num_iso_packets } as usize;

        let mut tx_fifo_level_min = 31;
        let mut tx_fifo_level_max = 0;

        for i in 0..num_iso_packets {
            let packet = unsafe { (*transfer).iso_packet_desc.get_unchecked_mut(i) };

            if packet.status == 0 {
                let buffer = unsafe {
                    let p = libusb_get_iso_packet_buffer(transfer, i.try_into().unwrap());
                    slice::from_raw_parts_mut(p, packet.actual_length.try_into().unwrap()) 
                };

                let (buffer, usb_report) = buffer.split_at(buffer.len() - size_of::<RxUSBReport>());
                let usb_report = bytemuck::from_bytes::<RxUSBReport>(usb_report);

                // Check that USB start-of-frame count is sequential. If frames were skipped
                // or repeated, make a note of it.
                if usb_report.sof_count != self.sof_count_next {
                    eprint!("S");
                }
                self.sof_count_next = usb_report.sof_count.wrapping_add(1);

                if usb_report.fifo_tx_level < tx_fifo_level_min {
                    tx_fifo_level_min = usb_report.fifo_tx_level;
                }
                if usb_report.fifo_tx_level > tx_fifo_level_max {
                    tx_fifo_level_max = usb_report.fifo_tx_level;
                }

                for frame_in in buffer.chunks_exact(size_of::<RxFrame>()) {
                    let frame_in = bytemuck::from_bytes::<RxFrame>(frame_in);

                    // Check that frame count is sequential. If frames were skipped
                    // or repeated, make a note of it.
                    if frame_in.report.frame_count != self.frame_count_next {
                        eprint!("F");
                    }
                    self.frame_count_next = frame_in.report.frame_count.wrapping_add(1);

                    let frame = InternalFrame {
                        frame: frame_in.frame,
                        frame_count: frame_in.report.frame_count,
                    };
                    if let Err(e) = self.frames_in.push(frame) {
                        eprint!("I");
                    }
                }
            }
        }

        unsafe { libusb_set_iso_packet_lengths(transfer, 512) };

        let result = unsafe {
            ffi::libusb_submit_transfer(transfer)
        };
        match result {
            LIBUSB_SUCCESS => {},
            e => eprintln!("IN: libusb_submit_transfer error: {e}"),
        }

        self.tx_fifo_level_min = tx_fifo_level_min;
    }
}

impl CallbackOut for LoopbackFrameHandler {
    fn callback_out(&mut self, transfer: *mut ffi::libusb_transfer) {
        let num_iso_packets = unsafe { (*transfer).num_iso_packets } as usize;

        if self.tx_fifo_level_min > 12 {
            // Simple way to draw down the TX FIFO level if it's too high.
            // We're dropping a frame here...
            let _ = self.frames_out.pop();
            eprint!("D");
        }

        for i in 0..num_iso_packets {
            let available_frames = self.frames_out.len();
            let frame_count = if available_frames > 2 {
                2
            } else {
                available_frames
            };

            let packet = unsafe { (*transfer).iso_packet_desc.get_unchecked_mut(i) };
            packet.length = (size_of::<TxUSBReport>() + frame_count * size_of::<TxFrame>()).try_into().unwrap();
            packet.actual_length = packet.length;

            let buffer = unsafe {
                let p = libusb_get_iso_packet_buffer(transfer, i.try_into().unwrap());
                slice::from_raw_parts_mut(p, packet.actual_length.try_into().unwrap()) 
            };

            let (usb_report, buffer) = buffer.split_at_mut(size_of::<TxUSBReport>());
            let usb_report = bytemuck::from_bytes_mut::<TxUSBReport>(usb_report);
            // usb_report.frame_count = ?;

            for frame in buffer.chunks_exact_mut(size_of::<TxFrame>()) {
                let frame = bytemuck::from_bytes_mut::<TxFrame>(frame);

                if let Some(frame_out) = self.frames_out.pop() {
                    frame.frame = frame_out.frame;
                    frame.report.frame_count = frame_out.frame_count;
                } else {
                    eprint!("O");
                }
            }
        }

        let result = unsafe {
            ffi::libusb_submit_transfer(transfer)
        };
        match result {
            LIBUSB_SUCCESS => {},
            e => eprintln!("OUT: libusb_submit_transfer error: {e}"),
        }
    }
}
