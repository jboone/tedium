use std::thread;
use std::time::Duration;

use clap::{Parser, Subcommand, Args, ArgEnum};

use console::{Color, style};
use crossbeam::channel::unbounded;
use framer::device::{Timeslot, Channel, AsyncThing};
use framer::default::{register_defaults, DefaultsMode};
use framer::test::{set_test_mode_liu, LIUTestMode, set_test_mode_framer, FramerTestMode};

use crate::framer::device::{Device, Result};
use crate::framer::register::*;

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

#[derive(Args)]
pub(crate) struct InitArgs {
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
    #[clap(name="init")]
    Init(InitArgs),

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
    let mut device = Device::open(&mut context).expect("device open");

    match args.command {
        Commands::Init(_) => {
            configure(&device)?;
        },
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
            // monitor(&context, &device)?;

            // if let Err(e) = framer::audio::pump3() {
            if let Err(e) = framer::audio::pump_loopback() {
                eprintln!("error: audio pump: {:?}", e);
            }
        },
    }

    Ok(())
}

///////////////////////////////////////////////////////////////////////

fn monitor_channel_configure(channel: &Channel) -> Result<()> {
    // Disable interrupts on all channels. Later, we'll enable
    // interrupts on the channel we want to watch and manage.
    channel.bier().write(|w| w
        .with_LBCODE_ENB(0)
        .with_RXCLKLOSS(0)
        .with_ONESEC_ENB(0)
        .with_HDLC_ENB(0)
        .with_SLIP_ENB(0)
        .with_ALARM_ENB(0)
        .with_T1FRAME_ENB(0)
    )?;

    // TODO: This is only here to test clock loss detection
    // channel.csr().modify(|m| m
    //     .with_Clock_Loss_Detect(0)
    // )?;

    // Enable interrupts and status

    // Enable receiver AIS detection
    channel.agr().modify(|m| m
        .with_Transmit_AIS_Pattern_Select(TransmitAISPattern::Disable)
        .with_AIS_Defect_Declaration_Criteria(AISDetection::UnframedAndFramed)
    )?;

    // Configure yellow alarm transmission
    // * One second rule
    channel.agr().modify(|m| m
        .with_Yellow_Alarm_One_Second_Rule(1)
        .with_ALARM_ENB(0)
        .with_YEL(0b01)
    )?;

    // Enable Customer Installation alarm detect (only in ESF)
    channel.ciagr().modify(|m| m
        // .with_CIAD(0b01)    // Enable unframed AIS-CI alarm detection
        .with_CIAD(0b10)    // Enable the RAI-CI alarm detection
    )?;

    // Enable interrupt status automatic reset-upon-read behavior
    // Enable interrupts from framer block.
    channel.icr().modify(|m| m
        .with_INT_WC_RUR(0)
        .with_ENBCLR(0)     // NOTE: This clears interrupt *ENABLES*, not *STATUSES*. Whoops.
        .with_INTRUP_ENB(1)
    )?;

    if false {
        // T1 Synchronization Status Message (SSM)
        // channel.boccr().modify(|m| m
        // )?;
    }

    // In-band loopback (not applicable for ESF, right?)
    // channel.lccr0().modify(|m| m
    //     .with_RXLBCALEN(0b11)
    //     .with_RXLBCDLEN(0b11)
    //     .with_TXLBCLEN(0b11)
    //     .with_FRAMED(1)
    //     .with_AUTOENB(1)
    // )?;
    // channel.rxlbac().modify(|m| m
    //     .with_RXLBAC(0b000_1110)
    //     .with_RXLBACEN(1)
    // )?;
    // channel.rxldcr().modify(|m| m
    //     .with_RXLBDC(0b011_1000)
    //     .with_RXLBDCEN(1)
    // )?;

    // channel.rifr().modify(|m| m
    //     .with_FRAlarmMask(1)    // "mask" terminology is very confusing!
    // )?;

    // channel.prcr().modify(|m| m
    //     .with_RLOS_OUT_ENB(1)
    // )?;

    // Enable all interrupts, even though it *seems* the status registers
    // reflect events even if interrupts aren't enabled?
    channel.bier().modify(|m| m
        .with_LBCODE_ENB(0)
        .with_RXCLKLOSS(0)
        .with_ONESEC_ENB(0)
        .with_HDLC_ENB(1)
        .with_SLIP_ENB(1)
        .with_ALARM_ENB(1)
        .with_T1FRAME_ENB(1)
    )?;

    // Alarm & Error Interrupts
    channel.aeier().modify(|m| m
        .with_LCV_ENB(1)
        .with_RxOOF_ENB(1)
        .with_RxAIS_ENB(1)
        .with_RxYEL_ENB(1)
    )?;

    // Framer Interrupts
    channel.fier().modify(|m| m
        .with_DS0_ENB(1)    // I think this is only possible with non-ESF framing?
        .with_SIG_ENB(1)
        .with_COFA_ENB(1)
        .with_OOF_ENB(1)
        .with_FMD_ENB(1)    // Frame mimic seems to happen with some frequency?
        .with_SE_ENB(1)     // CRC-6 synchronization doesn't seem directly actionable.
        .with_FE_ENB(1)     // Framing bit errors don't necessarily indicate that synchronization has been lost.
    )?;

    // Data Link (HDLC1) Interrupts
    channel.dlier1().modify(|m| m
        .with_TxSOT_ENB(0)      // For what it's worth, automatic reporting produces this interrupt.
        .with_RxSOT_ENB(0)
        .with_TxEOT_ENB(0)      // For what it's worth, automatic reporting produces this interrupt.
        .with_RxEOT_ENB(1)
        .with_FCS_ERR_ENB(0)    // Seems like status we should check on RxEOT, but doesn't require interrupting
        .with_RxIDLE_ENB(0)     // Not sure this has any value.
    )?;

    // Slip Buffer Interrupts
    // We'll keep a close eye on these for now, assuming my audio pump isn't very refined or well-behaved yet.
    channel.sbier().modify(|m| m
        .with_TxFULL_ENB(1)
        .with_TxEMPT_ENB(1)
        .with_TxSLIP_ENB(1)
        .with_RxFULL_ENB(1)
        .with_RxEMPT_ENB(1)
        .with_RxSLIP_ENB(1)
    )?;

    // Enable change in Excessive Zero condition interrupt?
    channel.exzer().modify(|m| m
        .with_EXZ_ENB(1)
    )?;

    // Enable SS7 for LAPD Controller 1 interrupt?
    channel.ss7er1().modify(|m| m
        .with_SS7_ENB(0)
    )?;

    // Enable Change in Receive LOS Condition interrupt?
    channel.rlcier().modify(|m| m
        .with_RxLOS_ENB(0)  // NOTE: Datasheet has the sense of this bit inverted! 0 = Enabled? Huh?
    )?;

    // Enable Change in Receive AIS-CI / RAI-CI Condition interrupt?
    channel.ciaier().modify(|m| m
        .with_RxAIS_CI_ENB(0)
        .with_RxRAI_CI_ENB(0)
    )?;

    // Enable T1 BOC interrupts?
    channel.bocier().modify(|m| m
        .with_RMTCH3(0)
        .with_RMTCH2(0)
        .with_BOCC(0)
        .with_RFDLAD(0)
        .with_RFDLF(0)
        .with_TFDLE(0)
        .with_RMTCH1(0)
        .with_RBOC(0)
    )?;

    // Enable T1 Unstable BOC SSM interrupts?
    channel.bocuier().modify(|m| m
        .with_Unstable(0)
    )?;

    // Enable LIU channel interrupts?
    channel.liuccier().modify(|m| m
        .with_DMOIE_n(0)
        .with_FLSIE_n(0)
        .with_LCVIE_n(0)   // Only for framer bypass operation.
        .with_NLCDIE_n(0)
        .with_AISDIE_n(0)   // Only for framer bypass operation.
        .with_RLOSIE_n(0)
        .with_QRPDIE_n(0)
    )?;

    let clear_interrupts = true;
    if clear_interrupts {
        // Clear any pending HDLC buffers.
        // NOTE: Didn't help unstick HDLC messages I was expecting to receive from myself via analog loopback.
        channel.rdlbcr1().read()?;
        for _ in 0..96 {
            channel.lapdbcr0(0).read()?;
            channel.lapdbcr1(0).read()?;
        }

        channel.dlsr1().read()?;
        channel.sbisr().read()?;

        channel.aeisr().read()?;
        channel.exzsr().read()?;
        channel.ciasr().read()?;

        channel.fisr().read()?;
    }

    Ok(())
}

#[derive(Copy, Clone, Debug, Default)]
struct ChannelStatus {
    sending_yellow_alarm: bool,
    receiving_yellow_alarm: bool,
}

fn monitor(context: &rusb::Context, device: &Device) -> Result<()> {
    let mut statuses = [ChannelStatus::default(); 8];

    for channel in device.channels() {
        monitor_channel_configure(&channel)?;

        let mut status = &mut statuses[channel.index()];

        status.sending_yellow_alarm = channel.ciagr().read()?.CIAG() == 0b10;
        if status.sending_yellow_alarm {
            println!("CH{} TX: already sending yellow alarm", channel.index());
        }
        status.receiving_yellow_alarm = channel.aeisr().read()?.RxYEL_State() != 0;
        if status.receiving_yellow_alarm {
            println!("CH{} RX: already receiving yellow alarm", channel.index());
        }
    }

    // if false {
    //     let mut last = 0;
    //     loop {
    //         let mut buf = [0; 4];
    //         channel.device().handle().read_interrupt(LIBUSB_ENDPOINT_IN | 9, &mut buf, Duration::MAX)?;
    //         let count = u32::from_be_bytes(buf);
    //         let diff = count - last;
    //         println!("{diff:9},");
    //         last = count;
    //     }
    // }

    let mut context = context.clone();
    // let handle = channel.device().handle();

    let (sender, receiver) = unbounded();
    thread::spawn(move || {
        if let Err(e) = AsyncThing::run(&mut context, sender) {
        // if let Err(e) = AsyncThing::run(&handle, sender) {
            eprintln!("error: async_thing: {:?}", e);
        } else {
            println!("async_thing done");
        }
    });


    println!("entering message loop");

    while let Ok(_) = receiver.recv() {
        // NOTE: If you enable interrupts for more than one channel, you must check and
        // clear registers for all interrupt-enabled channels, or you're going to have
        // the unmonitored channels producing interrupts that will hang up the single,
        // shared interrupt signal from the framer/LIU chip to the FPGA.

        // match channel.device().illegal_bagel(Duration::from_millis(100)) {
        //     Ok(4) => { println!("INTERRUPT!"); Ok(()) },
        //     Ok(_) => { println!("INTERRUPT! but unexpected length"); Ok(()) },
        //     Err(rusb::Error::Timeout) => { Ok(()) },
        //     Err(e) => Err(e),
        // }?;

        let color = |v| if v != 0 { Color::Red } else { Color::Green };

        for channel in device.channels() {

            let mut status = &mut statuses[channel.index()];

            let bisr = channel.bisr().read()?;

            // Clear the ONESEC interrupt, since it's not interesting *and* apparently
            // we can't disable it.
            let bisr = bisr.with_ONESEC(0);

            let bisr_has_interrupts = bisr.into_bytes()[0] != 0;

            if bisr_has_interrupts {
                print!("CH{} BISR:[{}][{}][{}][{}][{}][{}][{}]",
                    channel.index(),
                    style("LBCODE").fg(color(bisr.LBCODE())),
                    style("RXCLOS").fg(color(bisr.RxClkLOS())),
                    style("ONESEC").fg(color(bisr.ONESEC())),
                    style("HDLC").fg(color(bisr.HDLC())),
                    style("SLIP").fg(color(bisr.SLIP())),
                    style("ALRM").fg(color(bisr.ALARM())),
                    style("T1FRM").fg(color(bisr.T1FRAME())),
                );
            }

            if bisr.LBCODE() != 0 {
                // Loopback Code Block Interrupt Status
                // 
                // This bit indicates whether or not the Loopback Code block has an
                // interrupt request awaiting service.
                // 
                // 0 - Indicates no outstanding Loopback Code Block interrupt request
                // is awaiting service
                // 1 - Indicates the Loopback Code block has an interrupt request
                // awaiting service. Interrupt Service routine should branch to the inter-
                // rupt source and read the Loopback Code Interrupt Status register
                // (address 0xNB0A) to clear the interrupt
                // 
                // NOTE This bit will be reset to 0 after the microprocessor has
                // performed a read to the Loopback Code Interrupt Status Register.

                channel.rlcisr0().read()?;
            }

            if bisr.RxClkLOS() != 0 {
                // Loss of Recovered Clock Interrupt Status
                // This bit indicates whether or not the T1 receive framer is currently
                // declaring the "Loss of Recovered Clock" interrupt.
                // 
                // 0 = Indicates that the T1 Receive Framer Block is NOT currently
                // declaring the "Loss of Recovered Clock" interrupt.
                // 1 = Indicates that the T1 Receive Framer Block is currently declar-
                // ing the "Loss of Recovered Clock" interrupt.
                // 
                // NOTE : This bit is only active if the clock loss detection feature is
                // enabled (Register - 0xN100)
            }

            if bisr.ONESEC() != 0 {
                // One Second Interrupt Status
                // This bit indicates whether or not the T1 receive framer block is cur-
                // rently declaring the "One Second" interrupt.
                // 
                // 0 = Indicates that the T1 Receive Framer Block is NOT currently
                // declaring the "One Second" interrupt.
                // 1 = Indicates that the T1 Receive Framer Block is currently declar-
                // ing the "One Second" interrupt.
            }

            if bisr.HDLC() != 0 {
                // HDLC Block Interrupt Status
                // This bit indicates whether or not the HDLC block has any interrupt
                // request awaiting service.
                // 
                // 0 = Indicates no outstanding HDLC block interrupt request is await-
                // ing service
                // 1 = Indicates HDLC Block has an interrupt request awaiting service.
                // Interrupt Service routine should branch to the interrupt source and
                // read the corresponding Data LInk Status Registers (address
                // 0xNB06, 0xNB16, 0xNB26, 0xNB10, 0xNB18, 0xNB28) to clear the
                // interrupt.
                //
                // NOTE: This bit will be reset to 0 after the microprocessor has
                // performed a read to the corresponding Data Link Status
                // Registers that generated the interrupt.

                for i in 0..3 {
                    let dlsr = channel.dlsr(i).read()?;

                    // Mask off the MSG_TYPE and check if any of the remaining bits are set.
                    if dlsr.into_bytes()[0] & 0x7f != 0 {
                        print!(" DLSR{}:[{}][{}][{}][{}][{}][{}][{}][{}]",
                            i,
                            style("MOS").fg(color(dlsr.MSG_TYPE())),
                            style("TxSOT").fg(color(dlsr.TxSOT())),
                            style("RxSOT").fg(color(dlsr.RxSOT())),
                            style("TxEOT").fg(color(dlsr.TxEOT())),
                            style("RxEOT").fg(color(dlsr.RxEOT())),
                            style("FCS").fg(color(dlsr.FCS_ERR())),
                            style("RxABT").fg(color(dlsr.RxABORT())),
                            style("RxIDL").fg(color(dlsr.RxIDLE())),
                        );
                    }

                    if dlsr.RxEOT() != 0 && dlsr.RxIDLE() != 0 {
                        // If RxIDLE "AND" RxEOT occur together, then the entire
                        // HDLC message has been received.
                        
                        let rdlbcr = channel.rdlbcr(i).read()?;
                        let lapdbcr = match rdlbcr.RBUFPTR() {
                            0 => channel.lapdbcr0(0),
                            1 => channel.lapdbcr1(0),
                            _ => unreachable!(),
                        };

                        print!(" LAPDBCR{}:[", rdlbcr.RBUFPTR());
                        for _ in 0..rdlbcr.RDLBC() {
                            let value = lapdbcr.read()?;
                            print!("{value:02x}");
                        }
                        print!("]");

                        // TODO: What to do with dlsr1.FCS_ERR()?

                        if dlsr.FCS_ERR() != 0 {
                            print!(" FCS_ERR");
                        }
                    }

                    let _ = channel.ss7sr(i).read()?;
                }
            }

            if bisr.SLIP() != 0 {
                // Slip Buffer Block Interrupt Status
                // This bit indicates whether or not the Slip Buffer block has any out-
                // standing interrupt request awaiting service.
                // 
                // 0 = Indicates no outstanding Slip Buffer Block interrupt request is
                // awaiting service
                // 1 = Indicates Slip Buffer block has an interrupt request awaiting ser-
                // vice. Interrupt Service routine should branch to the interrupt source
                // and read the Slip Buffer Interrupt Status register (address 0xNB08)
                // to clear the interrupt
                // 
                // NOTE: This bit will be reset to 0 after the microprocessor has
                // performed a read to the Slip Buffer Interrupt Status
                // Register.

                let sbisr = channel.sbisr().read()?;

                print!(" SBISR:[{}][{}][{}][{}][{}][{}][{}][{}]",
                    style("TSBF").fg(color(sbisr.TxSB_FULL())),
                    style("TSBE").fg(color(sbisr.TxSB_EMPT())),
                    style("TSBS").fg(color(sbisr.TxSB_SLIP())),
                    style("RSBF").fg(color(sbisr.RxSB_FULL())),
                    style("RSBE").fg(color(sbisr.RxSB_EMPT())),
                    style("RSBS").fg(color(sbisr.RxSB_SLIP())),
                    style("SLC96LOCK").fg(color(sbisr.SLC96_LOCK())),
                    style("MFLOCK").fg(color(sbisr.Multiframe_LOCK())),
                );
            }

            if bisr.ALARM() != 0 {
                // Alarm & Error Block Interrupt Status
                // This bit indicates whether or not the Alarm & Error Block has any
                // outstanding interrupt request awaiting service.
                // 
                // 0 = Indicates no outstanding interrupt request is awaiting service
                // 1 = Indicates the Alarm & Error Block has an interrupt request await-
                // ing service. Interrupt service routine should branch to the interrupt
                // source and read the corresponding alarm and error status registers
                // (address 0xNB02, 0xNB0E, 0xNB40) to clear the interrupt.
                // 
                // NOTE: This bit will be reset to 0 after the microprocessor has
                // performed a read to the corresponding Alarm & Error
                // Interrupt Status register that generated the interrupt.

                let aeisr = channel.aeisr().read()?;
                let exzsr = channel.exzsr().read()?;
                let ciasr = channel.ciasr().read()?;

                print!(" AEISR:[{}][{}][{}][{}][{}][{}][{}][{}]",
                    style("RXOOF").fg(color(aeisr.RxOOF_State())),
                    style("RXAIS").fg(color(aeisr.RxAIS_State())),
                    style("RXYEL").fg(color(aeisr.RxYEL_State())),
                    style("LOS").fg(color(aeisr.LOS_State())),
                    style("LCV").fg(color(aeisr.LCVInt_Status())),
                    style("RXOOFX").fg(color(aeisr.RxOOF_State_Change())),
                    style("RXAISX").fg(color(aeisr.RxAIS_State_Change())),
                    style("RXYELX").fg(color(aeisr.RxYEL_State_Change())),
                );

                print!(" EXZSR:[{}]",
                    style("EXZ").fg(color(exzsr.EXZ_STATUS())),
                );

                print!(" CIASR:[{}][{}]",
                    style("RAISCI").fg(color(ciasr.RxAIS_CI_state())),
                    style("RRAICI").fg(color(ciasr.RxRAI_CI_state())),
                );

                // XRT86VL3X "Architecture Description", section 9.6 "T1 Brief discussion of alarms and error conditions"
                let rx_loss_of_frame = aeisr.RxOOF_State() != 0;
                let rx_loss_of_signal = aeisr.LOS_State() != 0;
                let remote_yellow_alarm = aeisr.RxYEL_State() != 0;

                let red_alarm = rx_loss_of_signal || rx_loss_of_frame;
                let send_yellow_alarm = red_alarm;

                if send_yellow_alarm && !status.sending_yellow_alarm {
                    channel.agr().modify(|m| m
                        .with_ALARM_ENB(1)
                    )?;
                    channel.ciagr().modify(|m| m
                        .with_CIAG(0b10)    // Enable the RAI-CI alarm generation
                    )?;
                    status.sending_yellow_alarm = true;
                    println!("CH{} TX: yellow alarm: start", channel.index());
                }

                if !send_yellow_alarm && status.sending_yellow_alarm {
                    channel.agr().modify(|m| m
                        .with_ALARM_ENB(0)
                    )?;
                    channel.ciagr().modify(|m| m
                        .with_CIAG(0b00)    // Disable the RAI-CI alarm generation
                    )?;
                    status.sending_yellow_alarm = false;
                    println!("CH{} TX: yellow alarm: stop", channel.index());
                }

                if remote_yellow_alarm && !status.receiving_yellow_alarm {
                    status.receiving_yellow_alarm = true;
                    println!("CH{} RX: yellow alarm: start", channel.index());
                }

                if !remote_yellow_alarm && status.receiving_yellow_alarm {
                    status.receiving_yellow_alarm = false;
                    println!("CH{} RX: yellow alarm: stop", channel.index());
                }
            }

            if bisr.T1FRAME() != 0 {
                // T1 Framer Block Interrupt Status
                // This bit indicates whether or not the T1 Framer block has any out-
                // standing interrupt request awaiting service.
                // 
                // 0 = Indicates no outstanding interrupt request is awaiting service.
                // 1 = Indicates the T1 Framer Block has an interrupt request awaiting
                // service. Interrupt service routine should branch to the interrupt
                // source and read the T1 Framer status register (address 0xNB04) to
                // clear the interrupt
                // 
                // NOTE: This bit will be reset to 0 after the microprocessor has
                // performed a read to the T1 Framer Interrupt Status register.

                let fisr = channel.fisr().read()?;

                print!(" FISR:[{}][{}][{}][{}][{}][{}][{}][{}]",
                    style("DS0X").fg(color(fisr.DS0_Change())),
                    style("DS0S").fg(color(fisr.DS0_Status())),
                    style("SIG").fg(color(fisr.SIG())),
                    style("COFA").fg(color(fisr.COFA())),
                    style("OOFX").fg(color(fisr.OOF_Status())),
                    style("FMD").fg(color(fisr.FMD())),
                    style("SE").fg(color(fisr.SE())),
                    style("FE").fg(color(fisr.FE())),
                );

                if fisr.SIG() != 0 {
                    let rscr_bitmap = channel.rscr_bitmap()?;
                    print!(" {:?}", rscr_bitmap);

                    for i in 0..rscr_bitmap.len() {
                        if rscr_bitmap.changed(i) {
                            let rsar = channel.rsar(i).read()?;
                            print!(" TS{i:02}[{}{}{}{}]", rsar.A(), rsar.B(), rsar.C(), rsar.D());
                        }
                    }
                }
            }

            if bisr_has_interrupts {
                println!();
            }

            // let liuccsr = channel.liuccsr().read()?;     // LIU channel current *status*
            let liuccisr = channel.liuccisr().read()?;  // LIU channel *change* in status. Read this to clear interrupts.
            let liugcr5 = channel.device().liugcr5().read()?;
            let bocisr = channel.bocisr().read()?;

            if liugcr5.into_bytes()[0] != 0 {
                println!("CH{} LIUGCR5:[{:?}]",
                    channel.index(),
                    liugcr5.into_bytes(),
                );
            }
            if liuccisr.into_bytes()[0] != 0 {
                println!("CH{} LIUCCISR:[{}][{}][{}][{}][{}][{}][{}]",
                    channel.index(),
                    style("DMOIS").fg(color(liuccisr.DMOIS_n())),
                    style("FLSIS").fg(color(liuccisr.FLSIS_n())),
                    style("LCVIS").fg(color(liuccisr.LCVIS_n())),
                    style("NLCDIS").fg(color(liuccisr.NLCDIS_n())),
                    style("AISDIS").fg(color(liuccisr.AISDIS_n())),
                    style("RLOSIS").fg(color(liuccisr.RLOSIS_n())),
                    style("QRPDIS").fg(color(liuccisr.QRPDIS_n())),
                );
            }
            if bocisr.into_bytes()[0] != 0 {
                println!("CH{} BOCISR:[{}][{}][{}][{}][{}][{}][{}][{}]",
                    channel.index(),
                    style("RMTCH3").fg(color(bocisr.RMTCH3())),
                    style("RMTCH2").fg(color(bocisr.RMTCH2())),
                    style("BOCC").fg(color(bocisr.BOCC())),
                    style("RFDLAD").fg(color(bocisr.RFDLAD())),
                    style("RFDLF").fg(color(bocisr.RFDLF())),
                    style("TFDLE").fg(color(bocisr.TFDLE())),
                    style("RMTCH1").fg(color(bocisr.RMTCH1())),
                    style("RBOC").fg(color(bocisr.RBOC())),
                );
            }

            // thread::sleep(Duration::from_millis(100));

            // match receiver.recv_timeout(Duration::from_millis(2000)) {
            //     Ok(AsyncThingMessage::Interrupt) => {},
            //     Err(RecvTimeoutError::Timeout) => {
            //         println!("<<< timeout >>>");
            //     },
            //     Err(RecvTimeoutError::Disconnected) => {
            //         eprintln!("error: receiver.recv(): disconnected");
            //         break;
            //     }
            // }
        }
    }

    Ok(())
}

///////////////////////////////////////////////////////////////////////

fn configure(device: &Device) -> Result<()> {
    register_defaults(device, DefaultsMode::Write)?;
    register_defaults(device, DefaultsMode::Check)?;

    configure_global(device)?;

    for channel in device.channels() {
        configure_channel(&channel)?;
    }

	device.framer_interface_control(true)?;

    // Globally enable interrupts, the last step to asserting the framer's INT# pin.
    device.liugcr0().modify(|m| m
        .with_GIE(1)
    )?;

    Ok(())
}

fn configure_global(device: &Device) -> Result<()> {
    device.liugcr4().write(|w| w
        .with_CLKSEL(ClockSelect::M16_384)
    )?;

    // TODO: PRCR (0xN11D) RLOS_OUT_ENB=0 to disable RLOS output pin (which isn't present on the 256-pin package)

    thread::sleep(Duration::from_millis(100));

    device.liugcr0().modify(|m| m
        .with_SRESET(1)
    )?;

    thread::sleep(Duration::from_millis(1000));

    device.liugcr0().modify(|m| m
        .with_SRESET(0)
    )?;
        
    thread::sleep(Duration::from_millis(100));

    Ok(())
}

fn configure_channel(channel: &Channel) -> Result<()> {
    // THEORY?
    // NOTE: I *think* the clock loss detection feature is not effective
    // in our case, as channels are currently configured to use TxSERCLK_n
    // as their transmit clock source ("External Timing Modee"). The FPGA
    // is "wired" to take the recovered clock from one of the channels and
    // mirror it to the TxSERCLK on all channels.

    channel.csr().write(|w| w
        .with_LCV_Insert(0)
        .with_Set_T1_Mode(1)
        .with_Sync_All_Transmitters_to_8kHz(0)
        .with_Clock_Loss_Detect(1)
        .with_CSS(ClockSource::External)
    )?;

    channel.licr().write(|w| w
        .with_FORCE_LOS(0)
        .with_Single_Rail_Mode(0)
        .with_LB(FramerLoopback::No)
        .with_Encode_B8ZS(0)
        .with_Decode_AMI_B8ZS(0)
    )?;

    channel.fsr().write(|w| w
        .with_Signaling_update_on_Superframe_Boundaries(1)
        .with_Force_CRC_Errors(0)
        .with_J1_MODE(0)
        .with_ONEONLY(1)    // Not the default, maybe more reliable sync?
        .with_FASTSYNC(0)
        .with_FSI(T1Framing::ExtendedSuperFrame)
    )?;

    channel.smr().write(|w| w
        .with_MFRAMEALIGN(0)    // Not used in base rate mode
        .with_MSYNC(0)    // Not used in base rate mode
        .with_Transmit_Frame_Sync_Select(0)
        .with_CRC6_Bits_Source_Select(0)
        .with_Framing_Bits_Source_Select(0)
    )?;

    channel.fcr().write(|w| w
        .with_Reframe(0)
        .with_Framing_with_CRC_Checking(1)
        .with_LOF_Tolerance(2)
        .with_LOF_Range(5)
    )?;

    // HDLC1 (for Facilities Data Link, right?)
    // Use "MOS" mode if we want 0b01111110 idle code with HDLC messages
    // (including reporting). Setting this makes the Adit 600s very happy,
    // stops the Adit from getting stuck when bringing up a channel.
    // Still gets stuck in payload loopback mode. Maybe it's important to
    // have MOS set *before* the channel starts sending frames, so that the
    // Adit doesn't autodetect(?) a BOS DLC channel instead of a MOS one?
    // So I've moved this to configure_channel().
    channel.dlcr1().modify(|m| m
        .with_SLC96_Data_Link_Enable(0)
        .with_MOS_ABORT_Disable(0)
        .with_Rx_FCS_DIS(0)
        .with_AutoRx(0)
        .with_Tx_ABORT(0)
        .with_Tx_IDLE(0)
        .with_Tx_FCS_EN(0)
        .with_MOS_BOSn(1)       
    )?;

    if true {
        // Performance reports
        channel.tsprmcr().modify(|m| m
            .with_FC_Bit(0)
            .with_PA_Bit(0)
            .with_U1_Bit(0)
            .with_U2_Bit(0)
            .with_R_Bit(0b0000)
        )?;
        channel.prcr().modify(|m| m
            .with_LBO_ADJ_ENB(0)
            .with_FAR_END(0)
            .with_NPRM(0b00)
            .with_C_R_Bit(0)
            .with_APCR(AutomaticPerformanceReport::EverySecond)
        )?;
    }

    channel.sbcr().write(|w| w
        .with_TxSB_ISFIFO(0)
        .with_SB_FORCESF(0)
        .with_SB_SFENB(0)
        .with_SB_SDIR(1)
        .with_SB_ENB(ReceiveSlipBuffer::SlipBuffer)
    )?;

    channel.ticr().write(|w| w
        .with_TxSyncFrD(0)
        .with_TxPLClkEnb_TxSync_Is_Low(0)
        .with_TxFr1544(0)
        .with_TxICLKINV(0)
        .with_TxIMODE(0b00)
    )?;

    channel.ricr().write(|w| w
        .with_RxSyncFrD(0)
        .with_RxPLClkEnb_RxSync_Is_Low(0)
        .with_RxFr1544(1)
        .with_RxICLKINV(0)
        .with_RxMUXEN(0)
        .with_RxIMODE(0b00)
    )?;

    channel.liuccr0().write(|w| w
        .with_QRSS_n_PRBS_n(PRBSPattern::PRBS)
        .with_PRBS_Rx_n_PRBS_Tx_n(PRBSDestination::TTIP_TRING)
        .with_RXON_n(1)
        .with_EQC(0x08)
    )?;

    channel.liuccr1().write(|w| w
        .with_RXTSEL_n(Termination::Internal)
        .with_TXTSEL_n(Termination::Internal)
        .with_TERSEL(TerminationImpedance::Ohms100)
        .with_RxJASEL_n(1)
        .with_TxJASEL_n(1)
        .with_JABW_n(0)
        .with_FIFOS_n(0)
    )?;

    channel.liuccr2().write(|w| w
        .with_INVQRSS_n(0)
        .with_TXTEST(TransmitTestPattern::None)
        .with_TXON_n(1)
        .with_LOOP2_n(LIULoopback::None)
    )?;

    for timeslot in channel.timeslots() {
        configure_timeslot(&timeslot)?;
    }

    Ok(())
}

fn configure_timeslot(timeslot: &Timeslot) -> Result<()> {
    timeslot.tccr().write(|w| w
        .with_LAPDcntl(TransmitLAPDSource::TSDLSR_TxDE)
        .with_TxZERO(ZeroCodeSuppression::None)
        .with_TxCOND(ChannelConditioning::Unchanged)
    )?;

    // Python code was using TUCR = 0, but seems like the chip default is fine or better.
    timeslot.tucr().write(|w| w
        .with_TUCR(0b0001_0111)
    )?;

    // Enable Robbed-Bit Signaling (RBS), using the flag contents in this register,
    // instead of the values coming in via the PCM serial interface.
    timeslot.tscr().write(|w| w
        .with_A_x(0)
        .with_B_y(1)
        .with_C_x(0)
        .with_D_x(1)
        .with_Rob_Enb(1)
        .with_TxSIGSRC(ChannelSignalingSource::TSCR)
    )?;

    timeslot.rccr().write(|w| w
        .with_LAPDcntl(ReceiveLAPDSource::RSDLSR_RxDE)
        .with_RxZERO(ZeroCodeSuppression::None)
        .with_RxCOND(ChannelConditioning::Unchanged)
    )?;

    timeslot.rucr().write(|w| w
        .with_RxUSER(0b1111_1111)
    )?;

    timeslot.rscr().write(|w| w
        .with_SIGC_ENB(0)
        .with_OH_ENB(0)
        .with_DEB_ENB(0)
        .with_RxSIGC(ReceiveSignalingConditioning::SixteenCode_ABCD)
        .with_RxSIGE(ReceiveSignalingExtraction::SixteenCode_ABCD)
    )?;

    timeslot.rssr().write(|w| w
        .with_SIG_16A_4A_2A(0)
        .with_SIG_16B_4B_2A(0)
        .with_SIG_16C_4A_2A(0)
        .with_SIG_16D_4B_2A(0)
    )?;

    Ok(())
}

///////////////////////////////////////////////////////////////////////

fn registers_dump_raw(device: &Device) -> Result<()> {
    for address in 0..=0xffff {
        let value = device.register_read(address).expect("register read");
        if address % 16 == 0 {
            print!("{address:04x}:");
        }
        print!(" {value:02x}");
        if address % 16 == 15 {
            println!();
        }
    }

    Ok(())
}

fn registers_dump_debug(device: &Device) -> Result<()> {
    registers_dump_global(device)?;

    for channel in device.channels() {
        registers_dump_channel(&channel)?;
    }

    Ok(())
}

fn registers_dump_global(device: &Device) -> Result<()> {
    println!("Device\tDEVID=0x{:02x?}, REVID=0x{:02x?}", device.devid().read()?.DEVID(), device.revid().read()?.REVID());

    println!("Global\t{:?}", device.liugcr0().read()?);
    println!("\t{:?}", device.liugcr1().read()?);
    println!("\t{:?}", device.liugcr2().read()?);
    println!("\t{:?}", device.liugcr3().read()?);
    println!("\t{:?}", device.liugcr4().read()?);
    println!("\t{:?}", device.liugcr5().read()?);

    Ok(())
}

fn registers_dump_channel(channel: &Channel) -> Result<()> {
    print!("CH {:1}", channel.index());
    println!("\t{:?}", channel.csr    ().read()?);
    println!("\t{:?}", channel.licr   ().read()?);
    println!("\t{:?}", channel.fsr    ().read()?);
    println!("\t{:?}", channel.agr    ().read()?);
    println!("\t{:?}", channel.smr    ().read()?);
    println!("\t{:?}", channel.tsdlsr ().read()?);
    println!("\t{:?}", channel.fcr    ().read()?);
    println!("\t{:?}", channel.rsdlsr ().read()?);
    println!("\t{:?}", channel.rscr0  ().read()?);
    println!("\t{:?}", channel.rscr1  ().read()?);
    println!("\t{:?}", channel.rscr2  ().read()?);
    println!("\t{:?}", channel.rifr   ().read()?);

    println!("\t{:?}", channel.sbcr   ().read()?);
    println!("\t{:?}", channel.fifolr ().read()?);
    // ...DMA...
    println!("\t{:?}", channel.icr    ().read()?);
    println!("\t{:?}", channel.lapdsr ().read()?);
    println!("\t{:?}", channel.ciagr  ().read()?);
    println!("\t{:?}", channel.prcr   ().read()?);
    println!("\t{:?}", channel.gccr   ().read()?);
    println!("\t{:?}", channel.ticr   ().read()?);

    println!("\t{:?}", channel.ricr   ().read()?);

    println!("\t{:?}", channel.tlcr   ().read()?);

    println!("\t{:?}", channel.rlcds  ().read()?);
    println!("\t{:?}", channel.dder   ().read()?);

    println!("\t{:?}", channel.tlcgs  ().read()?);
    println!("\t{:?}", channel.lcts   ().read()?);
    println!("\t{:?}", channel.tsprmcr().read()?);

    println!("\tHDLC1\t{:?}", channel.dlcr1   ().read()?);
    println!("\t\t{:?}",      channel.tdlbcr1 ().read()?);
    println!("\t\t{:?}",      channel.rdlbcr1 ().read()?);

    println!("\tHDLC2\t{:?}", channel.dlcr2   ().read()?);
    println!("\t\t{:?}",      channel.tdlbcr2 ().read()?);
    println!("\t\t{:?}",      channel.rdlbcr2 ().read()?);

    println!("\tHDLC3\t{:?}", channel.dlcr3   ().read()?);
    println!("\t\t{:?}",      channel.tdlbcr3 ().read()?);
    println!("\t\t{:?}",      channel.rdlbcr3 ().read()?);

    println!("\tLPC0\t{:?}", channel.lccr0   ().read()?);
    println!("\t\t{:?}",     channel.rlacr0  ().read()?);
    println!("\t\t{:?}",     channel.rldcr0  ().read()?);

    println!("\tLPC1\t{:?}", channel.lccr1   ().read()?);
    println!("\t\t{:?}",     channel.rlacr1  ().read()?);
    println!("\t\t{:?}",     channel.rldcr1  ().read()?);
    println!("\tLPC2\t{:?}", channel.lccr2   ().read()?);
    println!("\t\t{:?}",     channel.rlacr2  ().read()?);
    println!("\t\t{:?}",     channel.rldcr2  ().read()?);

    println!("\tLPC3\t{:?}", channel.lccr3   ().read()?);
    println!("\t\t{:?}",     channel.rlacr3  ().read()?);
    println!("\t\t{:?}",     channel.rldcr3  ().read()?);
    println!("\tLPC4\t{:?}", channel.lccr4   ().read()?);
    println!("\t\t{:?}",     channel.rlacr4  ().read()?);
    println!("\t\t{:?}",     channel.rldcr4  ().read()?);
    println!("\tLPC5\t{:?}", channel.lccr5   ().read()?);
    println!("\t\t{:?}",     channel.rlacr5  ().read()?);
    println!("\t\t{:?}",     channel.rldcr5  ().read()?);
    println!("\tLPC6\t{:?}", channel.lccr6   ().read()?);
    println!("\t\t{:?}",     channel.rlacr6  ().read()?);
    println!("\t\t{:?}",     channel.rldcr6  ().read()?);

    println!("\tLPC7\t{:?}", channel.lccr7   ().read()?);
    println!("\t\t{:?}",     channel.rlacr7  ().read()?);
    println!("\t\t{:?}",     channel.rldcr7  ().read()?);

    println!("\t{:?}",     channel.bcr     ().read()?);

    println!("\t{:?}",     channel.bertcsr0().read()?);

    println!("\t{:?}",     channel.bertcsr1().read()?);

    println!("\t{:?}",     channel.boccr   ().read()?);
    println!("\t{:?}",     channel.rfdlr   ().read()?);
    println!("\t{:?}",     channel.rfdlmr1 ().read()?);
    println!("\t{:?}",     channel.rfdlmr2 ().read()?);
    println!("\t{:?}",     channel.rfdlmr3 ().read()?);
    println!("\t{:?}",     channel.tfdlr   ().read()?);
    println!("\t{:?}",     channel.tbcr    ().read()?);

    print!("PM");
    println!("\t{:?}", channel.rlcvcu ().read()?);
    println!("\t{:?}", channel.rlcvcl ().read()?);
    println!("\t{:?}", channel.rfaecu ().read()?);
    println!("\t{:?}", channel.rfaecl ().read()?);
    println!("\t{:?}", channel.rsefc  ().read()?);
    println!("\t{:?}", channel.rsbbecu().read()?);
    println!("\t{:?}", channel.rsbbecl().read()?);
    println!("\t{:?}", channel.rsc    ().read()?);
    println!("\t{:?}", channel.rlfc   ().read()?);
    println!("\t{:?}", channel.rcfac  ().read()?);
    println!("\t{:?}", channel.lfcsec1().read()?);
    println!("\t{:?}", channel.pbecu  ().read()?);
    println!("\t{:?}", channel.pbecl  ().read()?);
    println!("\t{:?}", channel.tsc    ().read()?);
    println!("\t{:?}", channel.ezvcu  ().read()?);
    println!("\t{:?}", channel.ezvcl  ().read()?);
    println!("\t{:?}", channel.lfcsec2().read()?);
    println!("\t{:?}", channel.lfcsec3().read()?);

    print!("IRQ");
    println!("\t{:?}", channel.bisr   ().read()?);
    println!("\t{:?}", channel.bier   ().read()?);
    println!("\t{:?}", channel.aeisr  ().read()?);
    println!("\t{:?}", channel.aeier  ().read()?);
    println!("\t{:?}", channel.fisr   ().read()?);
    println!("\t{:?}", channel.fier   ().read()?);
    println!("\t{:?}", channel.dlsr1  ().read()?);
    println!("\t{:?}", channel.dlier1 ().read()?);
    println!("\t{:?}", channel.sbisr  ().read()?);
    println!("\t{:?}", channel.sbier  ().read()?);
    println!("\t{:?}", channel.rlcisr0().read()?);
    println!("\t{:?}", channel.rlcier0().read()?);
    println!("\t{:?}", channel.exzsr  ().read()?);
    println!("\t{:?}", channel.exzer  ().read()?);
    println!("\t{:?}", channel.ss7sr1 ().read()?);
    println!("\t{:?}", channel.ss7er1 ().read()?);
    println!("\t{:?}", channel.rlcisr ().read()?);
    println!("\t{:?}", channel.rlcier ().read()?);
    println!("\t{:?}", channel.rlcisr1().read()?);
    println!("\t{:?}", channel.rlcier1().read()?);
    println!("\t{:?}", channel.dlsr2  ().read()?);
    println!("\t{:?}", channel.dlier2 ().read()?);
    println!("\t{:?}", channel.ss7sr2 ().read()?);
    println!("\t{:?}", channel.ss7er2 ().read()?);
    println!("\t{:?}", channel.rlcisr2().read()?);
    println!("\t{:?}", channel.rlcier2().read()?);
    println!("\t{:?}", channel.rlcisr3().read()?);
    println!("\t{:?}", channel.rlcier3().read()?);
    println!("\t{:?}", channel.rlcisr4().read()?);
    println!("\t{:?}", channel.rlcier4().read()?);
    println!("\t{:?}", channel.rlcisr5().read()?);
    println!("\t{:?}", channel.rlcier5().read()?);
    println!("\t{:?}", channel.rlcisr6().read()?);
    println!("\t{:?}", channel.rlcier6().read()?);
    println!("\t{:?}", channel.rlcisr7().read()?);
    println!("\t{:?}", channel.rlcier7().read()?);
    println!("\t{:?}", channel.dlsr3  ().read()?);
    println!("\t{:?}", channel.dlier3 ().read()?);
    println!("\t{:?}", channel.ss7sr3 ().read()?);
    println!("\t{:?}", channel.ss7er3 ().read()?);
    println!("\t{:?}", channel.ciasr  ().read()?);
    println!("\t{:?}", channel.ciaier ().read()?);
    println!("\t{:?}", channel.bocisr ().read()?);
    println!("\t{:?}", channel.bocier ().read()?);
    println!("\t{:?}", channel.bocuisr().read()?);
    println!("\t{:?}", channel.bocuier().read()?);

    print!("LIU");
    println!("\t{:?}",     channel.liuccr0 ().read()?);
    println!("\t{:?}",     channel.liuccr1 ().read()?);
    println!("\t{:?}",     channel.liuccr2 ().read()?);
    println!("\t{:?}",     channel.liuccr3 ().read()?);
    println!("\t{:?}",     channel.liuccier().read()?);
    println!("\t{:?}",     channel.liuccsr ().read()?);
    println!("\t{:?}",     channel.liuccisr().read()?);
    println!("\t{:?}",     channel.liuccccr().read()?);
    println!("\t{:?}",     channel.liuccar1().read()?);
    println!("\t{:?}",     channel.liuccar2().read()?);
    println!("\t{:?}",     channel.liuccar3().read()?);
    println!("\t{:?}",     channel.liuccar4().read()?);
    println!("\t{:?}",     channel.liuccar5().read()?);
    println!("\t{:?}",     channel.liuccar6().read()?);
    println!("\t{:?}",     channel.liuccar7().read()?);
    println!("\t{:?}",     channel.liuccar8().read()?);

    for index in 0..24 {
        let timeslot = channel.timeslot(index);
        registers_dump_timeslot(&timeslot)?;
    }

    Ok(())
}

fn registers_dump_timeslot(timeslot: &Timeslot) -> Result<()> {
    println!("\tTS {:02}\t{:?}", timeslot.index(), timeslot.rds0mr().read()?);
    println!("\t\t{:?}", timeslot.tds0mr().read()?);
    println!("\t\t{:?}", timeslot.tccr  ().read()?);
    println!("\t\t{:?}", timeslot.tucr  ().read()?);
    println!("\t\t{:?}", timeslot.tscr  ().read()?);
    println!("\t\t{:?}", timeslot.rccr  ().read()?);
    println!("\t\t{:?}", timeslot.rucr  ().read()?);
    println!("\t\t{:?}", timeslot.rscr  ().read()?);
    println!("\t\t{:?}", timeslot.rssr  ().read()?);
    println!("\t\t{:?}", timeslot.rsar  ().read()?);

    Ok(())
}
