use std::collections::HashMap;
use std::mem::size_of;
use std::slice;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::thread;

use crate::codec::ulaw;
use crate::detector::{dtmf, Detector};
use crate::framer::device::open_device;
use crate::framer::usb::{InterfaceNumber, AlternateSetting, EndpointNumber, Transfer, CallbackIn, CallbackInWrapper, CallbackOut, CallbackOutWrapper};
use crate::generator::ToneGenerator;
use crate::generator::dual_tone::DualToneGenerator;

use audio_thread_priority::promote_current_thread_to_real_time;
use bytemuck::{Pod, Zeroable};
use crossbeam::channel::{unbounded, Sender, Receiver};
use ringbuf::{RingBuffer, Consumer, Producer};
use rusb::ffi::{libusb_set_iso_packet_lengths, libusb_get_iso_packet_buffer};
use rusb::{ffi, UsbContext};
use rusb::constants::{LIBUSB_TRANSFER_COMPLETED, LIBUSB_SUCCESS};

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

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct TimeslotAddress {
    channel: usize,
    timeslot: usize,
}

impl TimeslotAddress {
    pub fn new(channel: usize, timeslot: usize) -> Self {
        Self {
            channel,
            timeslot,
        }
    }
}

///////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum ToneSource {
    DialTonePrecise,
    Ringback,
}

#[derive(Copy, Clone, Debug)]
pub enum Patch {
    Idle,
    Input(TimeslotAddress),
    Tone(ToneSource),
}

struct Patching {
    map: [[Patch; CHANNELS]; TIMESLOTS_PER_CHANNEL],
}

impl Patching {
    fn timeslot(&self, address: &TimeslotAddress) -> &Patch {
        &self.map[address.timeslot][address.channel]
    }

    fn timeslot_mut(&mut self, address: &TimeslotAddress) -> &mut Patch {
        &mut self.map[address.timeslot][address.channel]
    }
}

impl Default for Patching {
    fn default() -> Self {
        use Patch::*;

        Self {
            map: [
                [Tone(ToneSource::DialTonePrecise), Idle, Idle, Idle, Idle, Idle, Idle, Idle,],  // Timeslot 00
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

#[derive(Copy, Clone, Debug)]
pub enum ProcessorMessage {
    Patch(TimeslotAddress, Patch),
}

struct AudioProcessor {
    patching: Patching,
    tone_plant: HashMap<ToneSource, Box<dyn ToneGenerator>>,
    detectors: HashMap<TimeslotAddress, Box<dyn Detector>>,
    message_receiver: Receiver<ProcessorMessage>,
}

impl AudioProcessor {
    fn new(message_receiver: Receiver<ProcessorMessage>) -> Self {
        let mut tone_plant: HashMap<ToneSource, Box<dyn ToneGenerator>> = HashMap::new();
        tone_plant.insert(ToneSource::DialTonePrecise, Box::new(DualToneGenerator::new(350.0, 440.0)));
        tone_plant.insert(ToneSource::Ringback, Box::new(DualToneGenerator::new(440.0, 480.0)));

        let mut detectors: HashMap<TimeslotAddress, Box<dyn Detector>> = HashMap::new();
        detectors.insert(TimeslotAddress::new(0, 0), Box::new(dtmf::Detector::new()));

        Self {
            patching: Patching::default(),
            tone_plant,
            detectors,
            message_receiver,
        }
    }

    fn process_message(&mut self, message: ProcessorMessage) {
        match message {
            ProcessorMessage::Patch(address, patch) => {
                *self.patching.timeslot_mut(&address) = patch;
            },
        }
    }

    fn process_frame(&mut self, frame_in: &Frame) -> Frame {
        while let Ok(message) = self.message_receiver.try_recv() {
            self.process_message(message);
        }

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
        let mut frame_out = Frame::default();
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
}

///////////////////////////////////////////////////////////////////////

pub fn pump_loopback(patch_receiver: Receiver<ProcessorMessage>) -> Result<(), PumpError> {
    let mut context = rusb::Context::new()?;

    let mut device = open_device(&mut context)?;

    use rusb::constants::*;
    
    device.claim_interface(InterfaceNumber::FrameStream as u8)?;
    device.set_alternate_setting(InterfaceNumber::FrameStream as u8, AlternateSetting::Active as u8)?;

    // TODO: Grab endpoint addresses from descriptors, instead of hard-coding them?

    let device = Arc::new(device);

    let device_handle = device;

    // For each USB transfer request (kernel URB?), get one millisecond worth of frames.
    // 8: Transfers are one USB-time millisecond long, nominally consisting of eight framer frames.
    const PACKETS_PER_TRANSFER: usize = 8;

    // Total number of transfers that can be in flight.
    const TRANSFERS_COUNT: usize = 8;

    let mut transfers_in: Vec<Transfer> = Vec::new();
    let mut transfers_out: Vec<Transfer> = Vec::new();

    let (debug_sender, debug_receiver) = unbounded();
    let handler = Arc::new(Mutex::new(LoopbackFrameHandler::new(patch_receiver, debug_sender)));

    for _ in 0..TRANSFERS_COUNT {
        let transfer_in = Transfer::new_iso_transfer(
            device_handle.clone(),
            LIBUSB_ENDPOINT_IN | EndpointNumber::FrameStream as u8,
            PACKETS_PER_TRANSFER,
            512,
            0,
            Box::new(CallbackInWrapper::new(handler.clone())),
        );

        transfer_in.submit();
        transfers_in.push(transfer_in);

        let transfer_out = Transfer::new_iso_transfer(
            device_handle.clone(),
            LIBUSB_ENDPOINT_OUT | EndpointNumber::FrameStream as u8,
            PACKETS_PER_TRANSFER,
            512,
            0,
            Box::new(CallbackOutWrapper::new(handler.clone())),
        );

        transfer_out.submit();
        transfers_out.push(transfer_out);
    }

    thread::Builder::new()
        .name("debug".into())
        .spawn(move || {
            let instant_start = Instant::now();
            let mut tx_fifo_level_range = (0, 0);

            for message in debug_receiver {
                match message {
                    DebugMessage::TxFIFORange(r) => {
                        if r != tx_fifo_level_range {
                            let elapsed = instant_start.elapsed();

                            let mut range_str = ['\u{2500}'; 32];
                            range_str[r.0 as usize] = '\u{2524}';
                            range_str[r.1 as usize] = '\u{251c}';
                            for i in (r.0 as usize)+1..(r.1 as usize) {
                                range_str[i] = ' ';
                            }
                            let range_str = range_str.iter().cloned().collect::<String>();

                            eprint!("\n{:6}.{:06}: {} ", elapsed.as_secs(), elapsed.subsec_micros(), range_str);
                            tx_fifo_level_range = r;
                        }
                    },
                    DebugMessage::FramerStatistics(p, c) => {
                        eprint!("\n{p:?} {c:?}");
                    },
                }
            }
        }).unwrap();

    // The current thread will be the one that handles all USB transfer callbacks.
    // So let's promote it to run more frequently and at a higher priority than
    // usual.
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

#[derive(Copy, Clone, Debug)]
enum DebugMessage {
    TxFIFORange((u8, u8)),
    FramerStatistics(FramerPeriodicStatistics, FramerCumulativeStatistics),
}

#[derive(Copy, Clone, Debug)]
#[repr(packed)]
struct Frame {
    timeslot: [[Sample; CHANNELS]; TIMESLOTS_PER_CHANNEL],
}

impl Frame {
    fn timeslot(&self, address: &TimeslotAddress) -> Sample {
        self.timeslot[address.timeslot][address.channel]
    }

    fn timeslot_mut(&mut self, address: &TimeslotAddress) -> &mut Sample {
        &mut self.timeslot[address.timeslot][address.channel]
    }
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            timeslot: [[0xff; CHANNELS]; TIMESLOTS_PER_CHANNEL],
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct InternalFrame {
    frame: Frame,
    frame_count: u32,
}

struct LoopbackFrameHandler {
    rx_packet_processor: RxPacketProcessor,
    unprocessed_frames_consumer: Consumer<InternalFrame>,
    processed_frames_producer: Producer<InternalFrame>,
    processed_frames_consumer: Consumer<InternalFrame>,
    processor: AudioProcessor,
    debug_sender: Sender<DebugMessage>,
}

impl LoopbackFrameHandler {
    fn new(processor_receiver: Receiver<ProcessorMessage>, debug_sender: Sender<DebugMessage>) -> Self {
        // 40 frames == 5 milliseconds.
        let (unprocessed_frames_producer, unprocessed_frames_consumer) = RingBuffer::new(40).split();
        let (processed_frames_producer, processed_frames_consumer) = RingBuffer::new(40).split();

        Self {
            rx_packet_processor: RxPacketProcessor::new(unprocessed_frames_producer, debug_sender.clone()),
            unprocessed_frames_consumer,
            processed_frames_producer,
            processed_frames_consumer,
            processor: AudioProcessor::new(processor_receiver),
            debug_sender,
        }
    }
}

/// Report received from framer over USB, after each frame of data.
/// Structure must match the one produced by the HDL on the FPGA.
#[derive(Copy, Clone, Debug)]
#[repr(packed)]
#[allow(dead_code)]
struct RxFrameReport {
    frame_count: u32,
}

#[derive(Copy, Clone, Debug)]
#[repr(packed)]
#[allow(dead_code)]
struct RxFrame {
    frame: Frame,
    report: RxFrameReport,
}

unsafe impl Zeroable for RxFrame {}
unsafe impl Pod for RxFrame {}

/// Report received from framer over USB, ideally at every USB
/// SOF (start-of-frame) interval. Structure must match the one
/// produced by the HDL on the FPGA.
#[derive(Copy, Clone, Debug)]
#[repr(packed)]
#[allow(dead_code)]
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
#[allow(dead_code)]
struct TxUSBReport {
    frame_count: u32,
}

unsafe impl Zeroable for TxUSBReport {}
unsafe impl Pod for TxUSBReport {}

#[derive(Copy, Clone, Debug)]
#[repr(packed)]
#[allow(dead_code)]
struct TxFrameReport {
    frame_count: u32,
}

#[derive(Copy, Clone, Debug)]
#[repr(packed)]
#[allow(dead_code)]
struct TxFrame {
    report: TxFrameReport,
    frame: Frame,
}

unsafe impl Zeroable for TxFrame {}
unsafe impl Pod for TxFrame {}

impl CallbackIn for LoopbackFrameHandler {
    fn callback_in(&mut self, transfer: *mut ffi::libusb_transfer) {
        self.handle_in(transfer);
    }
}

impl CallbackOut for LoopbackFrameHandler {
    fn callback_out(&mut self, transfer: *mut ffi::libusb_transfer) {
        self.handle_out(transfer);
    }
}

struct RxPacket<'a> {
    slice: &'a [u8],
}

impl<'a> RxPacket<'a> {
    fn from_slice(slice: &'a [u8]) -> Self {
        Self {
            slice,
        }
    }

    fn usb_report(&self) -> &RxUSBReport {
        let (_, usb_report) = self.slice.split_at(self.slice.len() - size_of::<RxUSBReport>());
        bytemuck::from_bytes::<RxUSBReport>(usb_report)
    }

    fn frames(&self) -> impl Iterator<Item=&RxFrame> {
        let (buffer, _) = self.slice.split_at(self.slice.len() - size_of::<RxUSBReport>());
        buffer.chunks_exact(size_of::<RxFrame>()).map(|b| bytemuck::from_bytes(b))
    }
}

const RX_FIFO_DEPTH: usize = 8;
const TX_FIFO_DEPTH: usize = 32;

#[derive(Copy, Clone, Debug)]
struct FramerPeriodicStatistics {
    rx_fifo_level_histogram: [u32; RX_FIFO_DEPTH],
    tx_fifo_level_histogram: [u32; TX_FIFO_DEPTH],
    frame_count: u32,
}

impl Default for FramerPeriodicStatistics {
    fn default() -> Self {
        Self {
            rx_fifo_level_histogram: [0; RX_FIFO_DEPTH],
            tx_fifo_level_histogram: [0; TX_FIFO_DEPTH],
            frame_count: 0,
        }
    }
}
#[derive(Copy, Clone, Debug)]
struct FramerCumulativeStatistics {
    rx_fifo_underflow_count: u16,
    tx_fifo_overflow_count: u16,
    sof_discontinuity_count: u32,
    frame_discontinuity_count: u32,
    ringbuf_full_drop_count: u32,
}

impl Default for FramerCumulativeStatistics {
    fn default() -> Self {
        Self {
            rx_fifo_underflow_count: 0,
            tx_fifo_overflow_count: 0,
            sof_discontinuity_count: 0,
            frame_discontinuity_count: 0,
            ringbuf_full_drop_count: 0,
        }
    }
}

struct RxPacketProcessor {
    unprocessed_frames_producer: Producer<InternalFrame>,
    framer_periodic_statistics: FramerPeriodicStatistics,
    framer_cumulative_statistics: FramerCumulativeStatistics,
    sof_count_next: u32,
    frame_count_next: u32,
    tx_fifo_level_min: u8,
    tx_fifo_level_max: u8,
    debug_sender: Sender<DebugMessage>,
}

impl RxPacketProcessor {
    fn new(frames_in: Producer<InternalFrame>, debug_sender: Sender<DebugMessage>) -> Self {
        Self {
            unprocessed_frames_producer: frames_in,
            framer_periodic_statistics: FramerPeriodicStatistics::default(),
            framer_cumulative_statistics: FramerCumulativeStatistics::default(),
            sof_count_next: 0,
            frame_count_next: 0,
            tx_fifo_level_min: u8::MAX,
            tx_fifo_level_max: u8::MIN,
            debug_sender,
        }
    }

    fn reset_tx_fifo_level_stats(&mut self) {
        self.tx_fifo_level_max = u8::MIN;
        self.tx_fifo_level_min = u8::MAX;
    }

    fn process(&mut self, packet: &RxPacket) {
        let usb_report = packet.usb_report();
        // Check that USB start-of-frame count is sequential. If frames were skipped
        // or repeated, make a note of it.
        if usb_report.sof_count != self.sof_count_next {
            self.framer_cumulative_statistics.sof_discontinuity_count += 1;
        }
        self.sof_count_next = usb_report.sof_count.wrapping_add(1);

        self.framer_periodic_statistics.rx_fifo_level_histogram[usb_report.fifo_rx_level as usize] += 1;
        self.framer_periodic_statistics.tx_fifo_level_histogram[usb_report.fifo_tx_level as usize] += 1;
        self.framer_cumulative_statistics.rx_fifo_underflow_count = usb_report.fifo_rx_underflow_count;
        self.framer_cumulative_statistics.tx_fifo_overflow_count = usb_report.fifo_tx_overflow_count;
        
        if usb_report.fifo_tx_level < self.tx_fifo_level_min {
            self.tx_fifo_level_min = usb_report.fifo_tx_level;
        }
        if usb_report.fifo_tx_level > self.tx_fifo_level_max {
            self.tx_fifo_level_max = usb_report.fifo_tx_level;
        }

        for frame_in in packet.frames() {
            self.framer_periodic_statistics.frame_count += 1;
            if self.framer_periodic_statistics.frame_count >= 8000 {
                self.debug_sender.send(DebugMessage::FramerStatistics(self.framer_periodic_statistics, self.framer_cumulative_statistics)).unwrap();
                self.framer_periodic_statistics = FramerPeriodicStatistics::default();
            }
    
            // Check that frame count is sequential. If frames were skipped
            // or repeated, make a note of it.
            if frame_in.report.frame_count != self.frame_count_next {
                self.framer_cumulative_statistics.frame_discontinuity_count += 1;
            }
            self.frame_count_next = frame_in.report.frame_count.wrapping_add(1);

            let frame = InternalFrame {
                frame: frame_in.frame,
                frame_count: frame_in.report.frame_count,
            };
            if let Err(_) = self.unprocessed_frames_producer.push(frame) {
                self.framer_cumulative_statistics.ringbuf_full_drop_count += 1;
            }
        }
    }
}

impl LoopbackFrameHandler {
    fn handle_in(&mut self, transfer: *mut ffi::libusb_transfer) {
        // TODO: Refactor stuff like this into code that only does the USB
        // work!
        let transfer_status = unsafe { (*transfer).status };
        if transfer_status != LIBUSB_TRANSFER_COMPLETED {
            eprintln!("IN: transfer.status = {transfer_status}");
        }

        let num_iso_packets = unsafe { (*transfer).num_iso_packets } as usize;

        self.rx_packet_processor.reset_tx_fifo_level_stats();

        for i in 0..num_iso_packets {
            // TODO: Wise to eliminate all non-essential work here so we can return
            // the transfer to the USB stack?
            
            let packet = unsafe { (*transfer).iso_packet_desc.get_unchecked_mut(i) };

            if packet.status == 0 {
                let buffer = unsafe {
                    let p = libusb_get_iso_packet_buffer(transfer, i.try_into().unwrap());
                    slice::from_raw_parts_mut(p, packet.actual_length.try_into().unwrap()) 
                };

                let rx_packet = RxPacket::from_slice(buffer);
                self.rx_packet_processor.process(&rx_packet);
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

        // Do non-essential stuff after we've returned the USB transfer
        // to the USB stack.
        self.debug_sender.send(DebugMessage::TxFIFORange(
            (self.rx_packet_processor.tx_fifo_level_min, self.rx_packet_processor.tx_fifo_level_max)
        )).unwrap();

        while let Some(unprocessed_frame) = self.unprocessed_frames_consumer.pop() {
            let processed_frame = self.processor.process_frame(&unprocessed_frame.frame);
            self.processed_frames_producer.push(InternalFrame {
                frame: processed_frame,
                frame_count: unprocessed_frame.frame_count,
            }).unwrap();
        }
    }

    fn handle_out(&mut self, transfer: *mut ffi::libusb_transfer) {
        let num_iso_packets = unsafe { (*transfer).num_iso_packets } as usize;

        if self.rx_packet_processor.tx_fifo_level_min <= self.rx_packet_processor.tx_fifo_level_max {
            // min/max are valid (not set to MAX/MIN).
            if self.rx_packet_processor.tx_fifo_level_min > 12 {
                // Simple way to draw down the TX FIFO level if it's too high.
                // We're dropping a frame here...
                let _ = self.processed_frames_consumer.pop();
                eprint!("D");
            }
        }

        for i in 0..num_iso_packets {
            let available_frames = self.processed_frames_consumer.len();
            let frame_count = if available_frames >= 2 {
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
            let _usb_report = bytemuck::from_bytes_mut::<TxUSBReport>(usb_report);
            // usb_report.frame_count = ?;

            for frame in buffer.chunks_exact_mut(size_of::<TxFrame>()) {
                let frame = bytemuck::from_bytes_mut::<TxFrame>(frame);

                if let Some(frame_out) = self.processed_frames_consumer.pop() {
                    frame.frame = frame_out.frame;
                    frame.report.frame_count = frame_out.frame_count;
                } else {
                    eprint!("O");
                }
            }
        }

        // TODO: Refactor stuff like this into code that only does the USB
        // work!
        let result = unsafe {
            ffi::libusb_submit_transfer(transfer)
        };
        match result {
            LIBUSB_SUCCESS => {},
            e => eprintln!("OUT: libusb_submit_transfer error: {e}"),
        }
    }
}
