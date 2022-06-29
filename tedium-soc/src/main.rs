#![no_std]
#![no_main]

extern crate panic_halt;

use embedded_hal::prelude::*;
// use riscv::register;
use riscv_rt::entry;

use xrt86vx38_pac::{self, device::{Result, Xyz, Channel, Timeslot}};
use xrt86vx38_pac::register::*;

mod framer;
mod peripheral;
mod test_points;
mod uart;
mod usb;

use framer::{Device, Access, FramerControl};
use test_points::TestPoints;
use uart::Uart;
use usb::{USBEndpointIn, USBEndpointOut};

fn configure_channel<D: Xyz>(channel: &Channel<D>) -> Result<()> {
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
        // Update RX RSAR and transmitted RBS only on superframe boundaries.
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

fn configure_timeslot<D: Xyz>(timeslot: &Timeslot<D>) -> Result<()> {
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
        // Enable RBS debounce on this timeslot
        .with_DEB_ENB(1)
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

fn configure(device: &Device) -> Result<()> {
    device.liugcr4().write(|w| w
        .with_CLKSEL(ClockSelect::M16_384)
    )?;

    for channel in device.channels() {
        configure_channel(&channel)?;
    }

    Ok(())
}

fn enable_interrupts<D: Xyz>(channel: &Channel<D>) -> Result<()> {
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
        channel.dlsr1().read()?;
        channel.dlsr2().read()?;
        channel.dlsr3().read()?;

        channel.rdlbcr1().read()?;
        channel.rdlbcr2().read()?;
        channel.rdlbcr3().read()?;

        for _ in 0..96 {
            channel.lapdbcr0(0).read()?;
            channel.lapdbcr1(0).read()?;
        }

        channel.ss7sr1().read()?;
        channel.ss7sr2().read()?;
        channel.ss7sr3().read()?;

        channel.sbisr().read()?;

        channel.aeisr().read()?;
        channel.exzsr().read()?;
        channel.ciasr().read()?;

        channel.fisr().read()?;
    }
    
    Ok(())
}

fn dump_registers<D: Xyz>(device: &D, uart: &Uart) {
    for s in 0..1 {
        for r in 0x100..0x200 {
            let address = (s << 12) + r;
            if address & 15 == 0 {
                uart.write_char(Uart::EOL);
                uart.write_hex_u16(address);
            }

            // let v = r & 0xff;
            let v = device.register_read(address as u16).unwrap();

            uart.write_char(Uart::SPACE);
            uart.write_hex_u8(v as u8);
        }

        uart.write_char(Uart::EOL);
    }
}

#[entry]
fn main() -> ! {
    let mut test_points = TestPoints::new(0x8000_2000);
    let framer_control = FramerControl::new(0x8000_3000);
    let device = Device::new(Access::new(0x8010_0000));
    let mut delay = riscv::delay::McycleDelay::new(60000000);
    let uart = Uart::new(0x8000_0000);
    let usb_in_interrupt = USBEndpointIn::new(0x8009_0000);
    let usb_out = USBEndpointOut::new(0x800a_0000);

    uart.write_str("reset\n");

    framer_control.set_outputs_control(false);
    framer_control.set_reset(true);

    delay.delay_us(50u16);

    framer_control.set_reset(false);

    delay.delay_us(50u16);

    dump_registers(&device, &uart);

    uart.write_str("configure\n");

    if configure(&device).is_err() {
        loop {}
    }

    framer_control.set_outputs_control(true);

    dump_registers(&device, &uart);

    for channel in device.channels() {
        enable_interrupts(&channel);
    }

    // Set true to mimic all interrupt types being asserted,
    // thereby sending all the current interrupt state and clearing
    // all pending interrupts.
    let mut resync_start = false;
    let mut resync = false;

    usb_out.set_ev_pending(usb_out.get_ev_pending());
    usb_out.set_ev_enable(1);
    usb_out.set_epno(3);
    usb_out.set_owner(1);
    usb_out.set_stall(0);
    usb_out.set_prime(1);
    usb_out.set_enable(1);

    loop {
        for (channel_index, channel) in device.channels().enumerate() {
            // Don't bother reading interrupt status until we can do something
            // about it -- meaning the USB endpoint is idle and we can transmit
            // data to the host.

            loop {
                if usb_in_interrupt.is_idle() {
                    break;
                }

                if usb_out.get_have() != 0 {
                    let ep = usb_out.get_data_ep();
                    uart.write_hex_u8(ep);
                    while usb_out.get_have() != 0 {
                        let data = usb_out.get_data();
                        uart.write_char(Uart::SPACE);
                        uart.write_hex_u8(data);
                    }
                    uart.write_char(Uart::EOL);

                    usb_out.set_stall(0);
                    usb_out.set_prime(1);
                    usb_out.set_enable(1);
                }
            }

            usb_in_interrupt.clear_stall();

            test_points.toggle(0);
            if channel_index == 0 {
                test_points.toggle(2);
            }

            if resync_start && channel_index == 0 {
                resync_start = false;
                resync = true;
            }

            let bisr = if resync {
                BISR::new()
                    .with_LBCODE(1)
                    .with_HDLC(1)
                    .with_SLIP(1)
                    .with_ALARM(1)
                    .with_T1FRAME(1)
            } else {
                channel.bisr().read().unwrap()
            };
            let bisr_u8: u8 = bisr.into();

            // Ignore the ONESEC interrupt, which apparently we can't shut off.
            if bisr_u8 & 0b01101111u8 != 0 {
                usb_in_interrupt.write_fifo(channel_index as u8);
                usb_in_interrupt.write_fifo(bisr_u8);

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

                    let rlcisr = channel.rlcisr().read().unwrap();
                    usb_in_interrupt.write_fifo(rlcisr.into());
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

                    for hdlc_index in 0..3 {
                        let dlsr = channel.dlsr(hdlc_index).read().unwrap();
                        usb_in_interrupt.write_fifo(dlsr.into());
                        if dlsr.RxEOT() != 0 { //&& dlsr.RxIDLE() != 0 {
                            let rdlbcr = channel.rdlbcr(hdlc_index).read().unwrap();
                            let rdlbc = rdlbcr.RDLBC();
                            let lapdbcr = match rdlbcr.RBUFPTR() {
                                0 => channel.lapdbcr0(0),
                                1 => channel.lapdbcr1(0),
                                _ => unreachable!(),
                            };
                            usb_in_interrupt.write_fifo(rdlbc);
                            for _ in 0..rdlbc {
                                let v = lapdbcr.read().unwrap();
                                usb_in_interrupt.write_fifo(v);
                            }
                        }

                        let _ = channel.ss7sr(hdlc_index).read();
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

                    let sbisr = channel.sbisr().read().unwrap();
                    usb_in_interrupt.write_fifo(sbisr.into());
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

                    let aeisr = channel.aeisr().read().unwrap();
                    usb_in_interrupt.write_fifo(aeisr.into());
                    let exzsr = channel.exzsr().read().unwrap();
                    usb_in_interrupt.write_fifo(exzsr.into());
                    let ciasr = channel.ciasr().read().unwrap();
                    usb_in_interrupt.write_fifo(ciasr.into());
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

                    let fisr = channel.fisr().read().unwrap();
                    usb_in_interrupt.write_fifo(fisr.into());
                    if fisr.SIG() != 0 {
                        for n in (0..24).step_by(2) {
                            let h: u8 = channel.rsar(n+0).read().unwrap().into();
                            let l: u8 = channel.rsar(n+1).read().unwrap().into();
                            let v = (h << 4) | (l & 0xf);
                            usb_in_interrupt.write_fifo(v);
                        }
                    }
                }
            }

            if !usb_in_interrupt.is_fifo_empty() {
                // Avoid sending ZLPs that wake up the host's USB handling thread
                // needlessly. The downside is a risk of a timeout if the USB stack
                // decides the interrupt endpoint has died?
                usb_in_interrupt.transmit(2);
            }

            if channel_index == 7 {
                resync = false;
            }
        }
    }
}
