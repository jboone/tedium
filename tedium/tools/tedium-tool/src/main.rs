use std::thread;

use clap::{Parser, Subcommand, Args, ArgEnum};

use crossbeam::channel::{unbounded, Receiver};
use framer::device::{FramerInterruptThread, FramerInterruptMessage};
use framer::dump::{registers_dump_raw, registers_dump_global, registers_dump_channel};
use framer::interrupt::{FramerInterruptStatus, print_framer_interrupt_status};
use framer::test::{set_test_mode_liu, LIUTestMode, set_test_mode_framer, FramerTestMode};

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
            let (framer_interrupt_sender, framer_interrupt_receiver) = unbounded();

            thread::Builder::new()
                .name("fr_int".to_string())
                .spawn(move || {
                    if let Err(e) = FramerInterruptThread::run(framer_interrupt_sender) {
                        eprintln!("error: framer interrupt pump: {e:?}");
                    }
                    eprintln!("done: framer interrupt pump");
                }).unwrap();

            thread::Builder::new()
                .name("fr_aud".to_string())
                .spawn(move || {
                    if let Err(e) = framer::audio::pump_loopback() {
                        eprintln!("error: audio pump: {:?}", e);
                    }
                    eprintln!("done: audio pump");
                }).unwrap();

            monitor(framer_interrupt_receiver);
            eprintln!("done: monitor");
        },
    }

    Ok(())
}

///////////////////////////////////////////////////////////////////////

fn monitor(receiver: Receiver<FramerInterruptMessage>) {
    while let Ok(m) = receiver.recv() {
        match m {
            FramerInterruptMessage::Interrupt(b, n) => {
                let truncated = &b[0..n];
                if let Ok(status) = FramerInterruptStatus::from_slice(truncated) {
                    print_framer_interrupt_status(&status);
                } else {
                    eprintln!("framer: interrupt: bad struct: {b:?}");
                }
            },
        }
    }
}
