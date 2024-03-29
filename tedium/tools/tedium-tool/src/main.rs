use std::thread;
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand, Args, ArgEnum};

use crossbeam::channel::{unbounded, Receiver};
use framer::FramerEvent;
use framer::interrupt::FramerInterruptThread;
use framer::dump::{registers_dump_raw, registers_dump_global, registers_dump_channel};
use framer::interrupt::{FramerInterruptStatus, print_framer_interrupt_status};
use framer::register::RSAR;
use framer::test::{set_test_mode_liu, LIUTestMode, set_test_mode_framer, FramerTestMode};

use crate::framer::audio::{TimeslotAddress, ProcessorMessage, Patch, ToneSource, DebugMessage};
use crate::framer::device::{Device, Result};

mod codec;
mod detector;
mod framer;
mod generator;

#[derive(Parser)]
#[clap(author, version, about, long_about=None)]
pub(crate) struct Cli {
    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(ArgEnum, Clone)]
pub(crate) enum TestMode {
    LIUDualLoopback,
    LIUAnalogLoopback,
    LIURemoteLoopback,
    LIUDigitalLoopback,
    FramerLocalLoopback,
    FramerRemoteLineLoopback,
    FramerPayloadLoopback,
}

#[derive(Args)]
pub(crate) struct TestArgs {
    #[clap(arg_enum)]
    mode: TestMode,

    #[clap(long)]
    pub channel: usize,
}

#[derive(Subcommand, Clone)]
pub(crate) enum DumpMode {
    #[clap(name="channel")]
    Channel {
        channel: usize,
    },

    #[clap(name="global")]
    Global,

    #[clap(name="all")]
    All,
}

#[derive(Args)]
pub(crate) struct DumpArgs {
    #[clap(subcommand)]
    mode: DumpMode,
}

#[derive(Args)]
pub(crate) struct MonitorArgs {
    // #[clap(long)]
    // pub channel: usize,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    #[clap(name="test")]
    Test(TestArgs),

    #[clap(name="dump")]
    Dump(DumpArgs),

    #[clap(name="monitor")]
    Monitor(MonitorArgs),
}

fn main() -> Result<()> {
    let args = Cli::parse();

    let mut context = rusb::Context::new()?;
    let device = Device::open(&mut context).expect("device open");

    match args.command {
        Commands::Test(a) => {
            let f = match a.mode {
                TestMode::LIUDualLoopback          => |c| set_test_mode_liu(c, LIUTestMode::DualLoopback),
                TestMode::LIUAnalogLoopback        => |c| set_test_mode_liu(c, LIUTestMode::AnalogLoopback),
                TestMode::LIURemoteLoopback        => |c| set_test_mode_liu(c, LIUTestMode::RemoteLoopback),
                TestMode::LIUDigitalLoopback       => |c| set_test_mode_liu(c, LIUTestMode::DigitalLoopback),
                TestMode::FramerLocalLoopback      => |c| set_test_mode_framer(c, FramerTestMode::LocalLoopback),
                TestMode::FramerRemoteLineLoopback => |c| set_test_mode_framer(c, FramerTestMode::RemoteLineLoopback),
                TestMode::FramerPayloadLoopback    => |c| set_test_mode_framer(c, FramerTestMode::PayloadLoopback),
            };

            let channel = device.channel(a.channel);
            f(&channel)?;
        },
        Commands::Dump(a) => {
            match a.mode {
                DumpMode::All => {
                    registers_dump_raw(&device)?;
                },
                DumpMode::Global => {
                    registers_dump_global(&device)?;
                },
                DumpMode::Channel { channel } => {
                    let channel = device.channel(channel);
                    registers_dump_channel(&channel)?;
                },
            }
        },
        Commands::Monitor(_) => {
            let (patch_sender, patch_receiver) = unbounded();
            let (event_sender, event_receiver) = unbounded();
            let (debug_sender, debug_receiver) = unbounded();

            thread::Builder::new()
                .name("fr_int".to_string())
                .spawn({
                    let event_sender = event_sender.clone();
                    move || {
                        if let Err(e) = FramerInterruptThread::run(event_sender) {
                            eprintln!("error: framer interrupt pump: {e:?}");
                        }
                        eprintln!("done: framer interrupt pump");
                    }
                }).unwrap();

            thread::Builder::new()
                .name("fr_aud".to_string())
                .spawn({
                    let event_sender = event_sender.clone();
                    move || {
                        if let Err(e) = framer::audio::pump_loopback(patch_receiver, event_sender, debug_sender) {
                            eprintln!("error: audio pump: {:?}", e);
                        }
                        eprintln!("done: audio pump");
                    }
                }).unwrap();

            const DEBUG_PRINT: bool = false;

            thread::Builder::new()
                .name("fr_dbg".into())
                .spawn(move || {
                    let instant_start = Instant::now();
                    let mut tx_fifo_level_range = (0, 0);

                    for message in debug_receiver {
                        match message {
                            DebugMessage::TxFIFORange(r) => {
                                if DEBUG_PRINT && r != tx_fifo_level_range {
                                    let elapsed = instant_start.elapsed();

                                    let mut range_str = ['\u{2500}'; 32];
                                    range_str[r.0 as usize] = '\u{2524}';
                                    range_str[r.1 as usize] = '\u{251c}';
                                    for i in (r.0 as usize)+1..(r.1 as usize) {
                                        range_str[i] = ' ';
                                    }
                                    let range_str = range_str.iter().cloned().collect::<String>();

                                    eprint!("{:6}.{:06}: {}\n", elapsed.as_secs(), elapsed.subsec_micros(), range_str);
                                    tx_fifo_level_range = r;
                                }
                            },
                            DebugMessage::FramerStatistics(p, c) => {
                                if DEBUG_PRINT {
                                    eprint!("{p:?} {c:?}\n");
                                }
                            },
                        }
                    }
                }).unwrap();
                
            thread::Builder::new()
                .name("repatch".into())
                .spawn(move || {
                    // Quick demo of sending changes to audio processor patching.
                    let address = TimeslotAddress::new(0, 0);

                    loop {
                        // Idle / on-hook.
                        patch_sender.send(ProcessorMessage::Patch(address, Patch::Idle)).unwrap();
                        thread::sleep(Duration::from_millis(1000));

                        // Dial tone
                        patch_sender.send(ProcessorMessage::Patch(address, Patch::Tone(ToneSource::DialTonePrecise))).unwrap();
                        thread::sleep(Duration::from_millis(1000));

                        // Ring / silence cadence.
                        for _ in 0..3 {
                            patch_sender.send(ProcessorMessage::Patch(address, Patch::Idle)).unwrap();
                            thread::sleep(Duration::from_millis(4000));
                            patch_sender.send(ProcessorMessage::Patch(address, Patch::Tone(ToneSource::Ringback))).unwrap();
                            thread::sleep(Duration::from_millis(2000));
                        }

                        // Connect to ourselves.
                        patch_sender.send(ProcessorMessage::Patch(address, Patch::Input(address))).unwrap();
                        thread::sleep(Duration::from_millis(5000));
                    }
                }).unwrap();

            monitor(event_receiver);
            eprintln!("done: monitor");
        },
    }

    Ok(())
}

///////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone, Debug)]
struct LineStateInterrupt {
    timestamp: Instant,
    abcd: u8,
}

impl Default for LineStateInterrupt {
    fn default() -> Self {
        Self {
            timestamp: Instant::now(),
            abcd: 0b0101,
        }
    }
}

impl LineStateInterrupt {
    fn set_state(&mut self, timestamp: Instant, rsar: RSAR) -> Option<(Duration, bool)> {
        let new_abcd = (rsar.A() << 3) | (rsar.B() << 2) | (rsar.C() << 1) | (rsar.D() << 0);
        if new_abcd != self.abcd {
            let duration = timestamp - self.timestamp;
            self.timestamp = timestamp;
            self.abcd = new_abcd;
            Some((duration, self.off_hook()))
        } else {
            None
        }
    }

    fn off_hook(&self) -> bool {
        self.abcd & 0x0c == 0b1100
    }
}

#[derive(Copy, Clone, Debug)]
struct LineStateRobbedBitState {
    frame_count: u32,
    abcd: u8,
}

impl Default for LineStateRobbedBitState {
    fn default() -> Self {
        Self {
            frame_count: 0,
            abcd: 0b0101,
        }
    }
}

impl LineStateRobbedBitState {
    fn set_state(&mut self, frame_count: u32, rbs_state: u8) -> Option<(Duration, bool)> {
        let new_abcd = rbs_state & 0x0f;
        if new_abcd != self.abcd {
            let frame_count_diff = frame_count.wrapping_sub(self.frame_count);
            let duration = Duration::from_millis((frame_count_diff / 8) as u64);
            self.frame_count = frame_count;
            self.abcd = new_abcd;
            Some((duration, self.off_hook()))
        } else {
            None
        }
    }

    fn off_hook(&self) -> bool {
        self.abcd & 0x0c == 0b1100
    }
}

fn monitor(receiver: Receiver<FramerEvent>) {
    let mut line_state_interrupt = [[LineStateInterrupt::default(); 24]; 8];
    let mut line_state_robbed_bit_state = [[LineStateRobbedBitState::default(); 24]; 8];

    while let Ok(m) = receiver.recv() {
        match m {
            FramerEvent::Interrupt { timestamp, data, length } => {
                let truncated = &data[0..length];
                if let Ok(status) = FramerInterruptStatus::from_slice(truncated) {
                    print_framer_interrupt_status(&status);

                    let channel_index = status.channel_index;

                    if let Some(t1frame) = status.t1frame {
                        if let Some(sig) = t1frame.sig {
                            for timeslot_index in 0..24 {
                                let rsar = sig.rsars[timeslot_index];
                                if let Some((duration, off_hook)) = line_state_interrupt[channel_index][timeslot_index].set_state(timestamp, rsar) {
                                    eprintln!("Interrupt: {channel_index}.{timeslot_index:02} {duration:?} {off_hook:?}");
                                }
                            }
                        }
                    }
                } else {
                    eprintln!("framer: interrupt: bad struct: {data:?}");
                }
            },
            FramerEvent::Digit(address, event) => {
                eprintln!("Digit {address:?}: {event:?}");
            },
            FramerEvent::RobbedBitState(frame_count, timeslot_address, rbs_state) => {
                let (channel_index, timeslot_index) = (timeslot_address.channel, timeslot_address.timeslot);
                if let Some((duration, off_hook)) = line_state_robbed_bit_state[timeslot_address.channel][timeslot_address.timeslot].set_state(frame_count, rbs_state) {
                    let duration_state_s = if off_hook { " on-hook" } else { "off-hook" };
                    let now_state_s = if off_hook { "off-hook" } else { "on-hook " };
                    eprintln!("RobbedBitState: {channel_index}.{timeslot_index:02} {duration_state_s} for {duration:?}, now {now_state_s}");
                }
            },
        }
    }
}
