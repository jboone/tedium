
use super::device::*;
use super::register::*;

pub(crate) enum FramerTestMode {
    LocalLoopback,
    RemoteLineLoopback,
    PayloadLoopback,
}

pub(crate) fn set_test_mode_framer(channel: &Channel, mode: FramerTestMode) -> Result<()> {
    match mode {
        FramerTestMode::LocalLoopback      => set_test_mode_framer_local_loopback(channel),
        FramerTestMode::RemoteLineLoopback => set_test_mode_framer_remote_line_loopback(channel),
        FramerTestMode::PayloadLoopback    => set_test_mode_framer_payload_loopback(channel),
    }
}

/// Framer local loopback
/// 
/// When framer local loopback is enabled, the transmit
/// PCM input data is looped back to the receive PCM out-
/// put data. The receive input data at RTIP/RRING is
/// ignored while an All Ones Signal is transmitted out to
/// the line interface.
/// 
/// ### At the line interface:
/// * RTIP/RRING -> ignored
/// * TTIP/TRING <- TAOS ("all ones")
/// 
/// ### At the serial interface:
/// * Serial TX drives Serial RX
/// * Serial RX <- RX Serial <- RX Slip <- RX Framer <- TX Framer <- TX Slip <- TX Serial
/// 
fn set_test_mode_framer_local_loopback(channel: &Channel) -> Result<()> {
    channel.licr().modify(|m| m
        .with_LB(FramerLoopback::Local)
    )?;

    // Use the RX recovered clock.
    channel.csr().modify(|m| m
        .with_CSS(ClockSource::Loop)
    )
}

/// Framer remote line loopback
/// 
/// The framer remote Line Loopback is almost identical to the LIU physical
/// interface Remote Loopback. The digital data enters the framer interface,
/// however does not enter the framing blocks. The main difference between
/// the Remote loopback and the Framer Remote Line loopback is that the
/// receive digital data from the LIU is allowed to pass through the LIU
/// Decoder/Encoder circuitry before returning to the line interface.
/// 
/// Any received unframed bitstream that has valid line coding should be re-transmitted error-free.
///
/// ### At the line interface:
/// * RTIP/RRING drives Framer RX and TTIP/TRING
/// * TTIP/TRING <- Timing Control <- TX Jitter Attenuator <- Encoder <- Framer RX <- Decoder <- RX Jitter Attenuator <- Data/Clock Recovery <- RTIP/RRING
/// 
/// ### At the LIU:Framer interface:
/// * Framer TX -> ignored.
/// * Framer RX <- Decoder <- RX Jitter Attenuator <- Data/Clock Recovery <- RTIP/RRING
/// 
fn set_test_mode_framer_remote_line_loopback(channel: &Channel) -> Result<()> {
    channel.licr().modify(|m| m
        .with_LB(FramerLoopback::FarEndRemoteLine)
    )
}

/// Framer payload loopback
/// 
/// When framer payload loopback is enabled, the raw
/// data within the receive time slots are looped back to the
/// transmit framer block where the data is re-framed
/// according to the transmit timing.
/// 
/// Any received  bitstream that has matching frame type should be re-transmitted error-free.
/// 
/// ### At the line interface:
/// * RTIP/RRING drives TTIP/TRING (and Framer RX?)
/// * TTIP/TRING <- TX Framer <- Timeslot Data Substitution <- RX Framer <- RTIP/RRING
/// 
/// ### At the serial interface:
/// * Serial TX provides timeslot data and TX timing
/// * Serial RX <- (RX Framer before timeslot data substitution?)
/// 
fn set_test_mode_framer_payload_loopback(channel: &Channel) -> Result<()> {
    channel.licr().modify(|m| m
        .with_LB(FramerLoopback::Payload)
    )?;

    // Use the RX recovered clock.
    channel.csr().modify(|m| m
        .with_CSS(ClockSource::Loop)
    )?;

    // If RBS is on, ESF+QRSS will show 1.04e-03 BER. Set TSCR[0-23].TxSIGSRC=0b00 and/or TSCR[0-23].Rob_Enb=0
    for timeslot in channel.timeslots() {
        timeslot.tscr().modify(|m| m
            .with_Rob_Enb(0)
            .with_TxSIGSRC(ChannelSignalingSource::PCMData)
        )?;
    }

    Ok(())
}

///////////////////////////////////////////////////////////////////////

pub(crate) enum LIUTestMode {
    DualLoopback,
    AnalogLoopback,
    RemoteLoopback,
    DigitalLoopback,
}

pub(crate) fn set_test_mode_liu(channel: &Channel, mode: LIUTestMode) -> Result<()> {
    match mode {
        LIUTestMode::DualLoopback    => set_test_mode_liu_dual_loopback(channel),
        LIUTestMode::AnalogLoopback  => set_test_mode_liu_analog_loopback(channel),
        LIUTestMode::RemoteLoopback  => set_test_mode_liu_remote_loopback(channel),
        LIUTestMode::DigitalLoopback => set_test_mode_liu_digital_loopback(channel),
    }
}

/// LIU dual loopback
/// 
/// Reconfigure a channel for LIU dual loopback.
/// 
/// Any received unframed bitstream that has valid(?) line coding should be re-transmitted error-free.
/// 
/// ### At the line interface:
/// * RTIP/RRING drives TTIP/TRING
/// * TTIP/TRING <- RX Jitter Attenuator <- Data/Clock Recovery <- RTIP/RRING
/// 
/// ### At the LIU:Framer interface:
/// * Framer TX drives Framer RX
/// * Framer RX <- Decoder <- TX Jitter Attenuator <- Encoder <- Framer TX
/// 
fn set_test_mode_liu_dual_loopback(channel: &Channel) -> Result<()> {
    channel.liuccr2().modify(|m| m
        .with_LOOP2_n(LIULoopback::Dual)
    )
}

/// LIU analog loopback
/// 
/// Reconfigure a channel for LIU analog loopback.
/// 
/// ### At the line interface:
/// * RTIP/RRING -> ignored
/// * TTIP/TRING <- Timing Control <- TX Jitter Attenuator <- Encoder <- Framer TX
/// 
/// ### At the LIU:Framer interface:
/// * Framer TX drives TTIP/TRING and Framer RX
/// * Framer RX <- Decoder <- RX Jitter Attenuator <- Data/Clock Recovery <- TTIP/TRING
/// 
fn set_test_mode_liu_analog_loopback(channel: &Channel) -> Result<()> {
    channel.liuccr2().modify(|m| m
        .with_LOOP2_n(LIULoopback::Analog)
    )
}

/// LIU remote loopback
/// 
/// Reconfigure a channel for LIU remote loopback.
/// 
/// Any received unframed bitstream that has valid(?) line coding should be re-transmitted error-free.
///
/// ### At the line interface:
/// * RTIP/RRING drives TTIP/TRING and Framer RX
/// * TTIP/TRING <- RX Jitter Attenuator <- Data/Clock Recovery <- RTIP/RRING
///  
/// At the LIU:Framer interface:
/// * Framer TX -> ignored.
/// * Framer RX <- Decoder <- RX Jitter Attenuator <- Data/Clock Recovery <- RTIP/RRING
/// 
fn set_test_mode_liu_remote_loopback(channel: &Channel) -> Result<()> {
    channel.liuccr2().modify(|m| m
        .with_LOOP2_n(LIULoopback::Remote)
    )
}

/// LIU digital loopback
/// 
/// Reconfigure a channel for LIU digital loopback.
/// 
/// ### At the line interface:
/// * RTIP/RRING -> ignored
/// * TTIP/TRING <- Timing Control <- TX Jitter Attenuator <- Encoder <- Framer TX
/// 
/// ### At the LIU:Framer interface:
/// * Framer TX drives Framer RX and TTIP/TRING
/// * Framer RX <- Decoder <- TX Jitter Attenuator <- Encoder <- Framer TX
/// 
fn set_test_mode_liu_digital_loopback(channel: &Channel) -> Result<()> {
    channel.liuccr2().modify(|m| m
        .with_LOOP2_n(LIULoopback::Digital)
    )
}
