#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(clippy::upper_case_acronyms)]

use modular_bitfield_msb::prelude::*;

///////////////////////////////////////////////////////////////////////
//

#[derive(Copy, Clone, BitfieldSpecifier, Debug)]
#[bits=2]
pub enum ClockSource {
    Loop = 0b00,
    External = 0b01,
    Internal = 0b10,
}

/// Clock Select Register (CSR) - 0xN100
/// 
/// * LCV Insert: Line Code Violation Insertion
///   This bit is used to force a Line Code Violation (LCV) on the transmit
///   output of TTIP/TRING.
///   A “0” to “1” transition on this bit will cause a single LCV to be inserted
///   on the transmit output of TTIP/TRING.
/// * Set T1 Mode: T1 Mode select
///   This bit is used to program the individual channel to operate in either
///   T1 or E1 mode.
///   0 = Configures the selected channel to operate in E1 mode.
///   1 = Configures the selected channel to operate in T1 mode.
/// * Sync All Transmitters to 8kHz:
///   This bit permits the user to configure the Transmit T1 Framer block to
///   synchronize its “transmit output” frame alignment with the 8kHz signal
///   that is derived from the MCLK PLL, as described below.
///   0 - Disables the “Sync all Transmit Framers to 8kHz” feature.
///   1 - Enables the “Sync all Transmit Framers to 8kHz” feature.
///   NOTE : This bit is only active if the MCLK PLL is used as the “Timing
///   Source” for the Transmit T1 Framer” blocks. CSS[1:0] of this
///   register allows users to select the transmit source of the
///   framer.
/// * Clock Loss Detect: Clock Loss Detect Enable/Disable Select
///   This bit enables a clock loss protection feature for the Framer when-
///   ever the recovered line clock is used as the timing source for the trans-
///   mit section. If the LIU loses clock recovery, the Clock Distribution Block
///   will detect this occurrence and automatically begin to use the internal
///   clock derived from MCLK PLL as the Transmit source, until the LIU is
///   able to regain clock recovery.
///   0 = Disables the clock loss protection feature.
///   1 = Enables the clock loss protection feature.
///   NOTE : This bit needs to be enabled in order to detect the clock closs
///   detection interrupt status (address: 0xNB00, bit 5)
/// * CSS: Clock Source Select
///   These bits select the timing source for the Transmit T1 Framer block.
///   These bits can also determine the direction of TxSERCLK, TxSYNC,
///   and TxMSYNC in base rate operation mode (1.544MHz Clock mode).
///   In Base Rate (1.544MHz Clock Mode):
///   | CSS[1:0] | TRANSMIT SOURCE FOR THE TRANSMIT T1 FRAMER BLOCK | DIRECTION OF TX SERCLK |
///   |----------|--------------------------------------------------|------------------------|
///   |  00/11   | _Loop Timing Mode_ The recovered line clock is chosen as the timing source. | Output |
///   |   01     | _External Timing Mode_ The Transmit Serial Input Clock from the TxSERCLK_n input pin is chosen as the timing source. | Input |
///   |   10     | _Internal Timing Mode_ The MCLK PLL is chosen as the timing source. | Output |
///   
///   _NOTE_: TxSYNC/TxMSYNC can be programmed as input or output
///   depending on the setting of SYNC INV bit in Register Address
///   0xN109, bit 4. Please see Register Description for the
///   Synchronization Mux Register (SMR - 0xN109) Table 10.
///   _NOTES_: In High-Speed or multiplexed modes, TxSERCLK, TxSYNC,
///   and TxMSYNC are all configured as INPUTS only.
///
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct CSR {
    pub LCV_Insert: B1,
    pub Set_T1_Mode: B1,
    pub Sync_All_Transmitters_to_8kHz: B1,
    pub Clock_Loss_Detect: B1,
    #[skip] __: B2,
    pub CSS: ClockSource,
}

impl Default for CSR {
    fn default() -> Self {
        CSR::from(0b0001_0001)
    }
}

#[derive(Copy, Clone, BitfieldSpecifier, Debug)]
#[bits=2]
pub enum FramerLoopback {
    /// No Loopback
    No = 0b00,

    /// ### Framer Local Loopback
    /// When framer local loopback is enabled, the transmit
    /// PCM input data is looped back to the receive PCM out-
    /// put data. The receive input data at RTIP/RRING is
    /// ignored while an All Ones Signal is transmitted out to
    /// the line interface.
    Local = 0b01,

    /// ### Framer Far-End (Remote) Line Loopback
    /// When framer remote loopback is enabled, the digital
    /// data enters the framer interface, however does not
    /// enter the framing blocks. The receive digital data from
    /// the LIU is allowed to pass through the LIU Decoder/
    /// Encoder circuitry before returning to the line interface.
    FarEndRemoteLine = 0b10,

    /// ### Framer Payload Loopback
    /// When framer payload loopback is enabled, the raw
    /// data within the receive time slots are looped back to the
    /// transmit framer block where the data is re-framed
    /// according to the transmit timing.
    Payload = 0b11,
}

/// Line Interface Control Register (LICR) - 0xN101
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct LICR {
    pub FORCE_LOS: B1,
    pub Single_Rail_Mode: B1,
    pub LB: FramerLoopback,
    #[skip] __: B2,
    pub Encode_B8ZS: B1,
    pub Decode_AMI_B8ZS: B1,
}

#[derive(Copy, Clone, BitfieldSpecifier, Debug)]
#[bits=3]
pub enum T1Framing {
    ExtendedSuperFrame = 0b000,
    SuperFrame = 0b101,
    N = 0b110,
    T1DM = 0b111,
    SLC96 = 0b100,
}

/// Framing Select Register (FSR) - 0xN107
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct FSR {
    pub Signaling_update_on_Superframe_Boundaries: B1,
    pub Force_CRC_Errors: B1,
    pub J1_MODE: B1,
    pub ONEONLY: B1,
    pub FASTSYNC: B1,
    pub FSI: T1Framing,
}

#[derive(Copy, Clone, BitfieldSpecifier, Debug)]
#[bits=2]
pub enum TransmitAISPattern {
    Disable = 0b00,
    Unframed = 0b01,
    Framed = 0b11,
}

#[derive(Copy, Clone, BitfieldSpecifier, Debug)]
#[bits=2]
pub enum AISDetection {
    Disabled = 0b00,
    UnframedAndFramed = 0b01,
    Framed = 0b11,
}

/// Alarm Generation Register (AGR) - 0xN108
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct AGR {
    pub Yellow_Alarm_One_Second_Rule: B1,
    pub ALARM_ENB: B1,
    pub YEL: B2,
    pub Transmit_AIS_Pattern_Select: TransmitAISPattern,
    pub AIS_Defect_Declaration_Criteria: AISDetection,
}

/// Synchronization MUX Register (SMR) - 0xN109
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct SMR {
    #[skip] __: B1,
    pub MFRAMEALIGN: B1,
    pub MSYNC: B1,
    pub Transmit_Frame_Sync_Select: B1,
    #[skip] __: B2,
    pub CRC6_Bits_Source_Select: B1,
    pub Framing_Bits_Source_Select: B1,
}

/// Transmit Signaling and Data Link Select Register (TSDLSR) - 0xN10a
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct TSDLSR {
    #[skip] __: B2,
    pub TxDLBW: B2,
    pub TxDE: B2,
    pub TxDL: B2,
}

/// Framing Control Register (FCR) - 0xN10b
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct FCR {
    pub Reframe: B1,
    pub Framing_with_CRC_Checking: B1,
    pub LOF_Tolerance: B3,
    pub LOF_Range: B3,
}

/// Receive Signaling and Data Link Select Register (RSDLSR) - 0xN10c
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct RSDLSR {
    #[skip] __: B2,
    pub RxDLBW: B2,
    pub RxDE: B2,
    pub RxDL: B2,
}

/// Receive Signaling Change Registers 0 (RSCR0) - 0xN10d
/// Receive Signaling Change Registers 1 (RSCR1) - 0xN10e
/// Receive Signaling Change Registers 2 (RSCR2) - 0xN10f
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct RSChR {
    pub Ch: B8,
}

// pub struct ReceiveSignalingChange {
//     cas_changed: [u8; 3],
// }

// impl ReceiveSignalingChange {
//     pub fn from_bytes(b: [u8; 3]) -> Self {
//         Self {
//             cas_changed: b,
//         }
//     }
// }

// impl fmt::Debug for ReceiveSignalingChange {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "ReceiveSignalingChange {{ 0b{:08b}_{:08b}_{:08b} }}", self.cas_changed[0], self.cas_changed[1], self.cas_changed[2])
//     }
// }

/// Receive In Frame Register (RIFR) - 0xN112
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct RIFR {
    pub In_Frame: B1,
    #[skip] __: B1,
    pub AIS_Ingress: B1,
    pub FRAlarmMask: B1,
    pub DS0Yel: B1,
    pub DS0Yel_Switch: B1,
    #[skip] __: B2,
}

/// Data Link Control Register (DLCR1) - 0xN113
/// Data Link Control Register (DLCR2) - 0xN143
/// Data Link Control Register (DLCR3) - 0xN153
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct DLCR {
    pub SLC96_Data_Link_Enable: B1,
    pub MOS_ABORT_Disable: B1,
    pub Rx_FCS_DIS: B1,
    pub AutoRx: B1,
    pub Tx_ABORT: B1,
    pub Tx_IDLE: B1,
    pub Tx_FCS_EN: B1,
    pub MOS_BOSn: B1,
}

/// Transmit Data Link Byte Count Register (TDLBCR1) - 0xN114
/// Transmit Data Link Byte Count Register (TDLBCR2) - 0xN144
/// Transmit Data Link Byte Count Register (TDLBCR3) - 0xN154
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct TDLBCR {
    pub TxHDLC_BUFAvail_BUFSel: B1,
    pub TDLBC: B7,
}

/// Receive Data Link Byte Count Register (RDLBCR1) - 0xN115
/// Receive Data Link Byte Count Register (RDLBCR2) - 0xN145
/// Receive Data Link Byte Count Register (RDLBCR3) - 0xN155
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct RDLBCR {
    pub RBUFPTR: B1,
    pub RDLBC: B7,
}

#[derive(Copy, Clone, BitfieldSpecifier, Debug)]
#[bits=2]
pub enum ReceiveSlipBuffer {
    Bypass = 0b00,
    SlipBuffer = 0b01,
    FIFO = 0b10,
}

/// Slip Buffer Control Register (SBCR) - 0xN116
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct SBCR {
    pub TxSB_ISFIFO: B1,
    #[skip] __: B2,
    pub SB_FORCESF: B1,
    pub SB_SFENB: B1,
    pub SB_SDIR: B1,
    pub SB_ENB: ReceiveSlipBuffer,
}

/// FIFO Latency Register (FIFOLR) - 0xN117
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct FIFOLR {
    #[skip] __: B3,
    pub Rx_Slip_Buffer_FIFO_Latency: B5,
}

// TODO: Skipping the DMA registers... D0WCR, D1RCR

/// Interrupt Control Register (ICR) - 0xN11a
///
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct ICR {
    #[skip] __: B5,
    pub INT_WC_RUR: B1,
    pub ENBCLR: B1,
    pub INTRUP_ENB: B1,
}

/// LAPD Select Register (LAPDSR) - 0xN11b
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct LAPDSR {
    #[skip] __: B3,
    pub HDLC3en: B1,
    pub HDLC2en: B1,
    pub HDLC1en: B1,
    pub HDLC_Controller_Select: B2,
}

/// Customer Installation Alarm Generation Register (CIAGR) - 0xN11c
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct CIAGR {
    #[skip] __: B4,
    pub CIAG: B2,
    pub CIAD: B2,
}

#[derive(Copy, Clone, BitfieldSpecifier, Debug)]
#[bits=2]
pub enum AutomaticPerformanceReport {
    No = 0b00,
    Once = 0b01,
    EverySecond = 0b10,
}

/// Performance Report Control Register (PRCR) - 0xN11d
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct PRCR {
    pub LBO_ADJ_ENB: B1,
    pub RLOS_OUT_ENB: B1,
    pub FAR_END: B1,
    pub NPRM: B2,
    pub C_R_Bit: B1,
    pub APCR: AutomaticPerformanceReport,
}

/// Gapped Clock Control Register (GCCR) - 0xN11e
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct GCCR {
    pub FrOutclk: B1,
    #[skip] __: B5,
    pub TxGCCR: B1,
    pub RxGCCR: B1,
}

/// Transmit Interface Control Register (TICR) - 0xN120
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct TICR {
    pub TxSyncFrD: B1,
    #[skip] __: B1,
    pub TxPLClkEnb_TxSync_Is_Low: B1,
    pub TxFr1544: B1,
    pub TxICLKINV: B1,
    pub TxMUXEN: B1,
    pub TxIMODE: B2,
}

#[derive(Copy, Clone, BitfieldSpecifier, Debug)]
#[bits=2]
pub enum BitErrorInsertionRate {
    Disable = 0b00,
    OneOfOneThousand,
    OneOfOneMillion,
}

/// BERT Control and Status Register (BERTCSR0) - 0xN121
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct BERTCSR0 {
    #[skip] __: B4,
    pub BERT_Switch: B1,
    pub BER: BitErrorInsertionRate,
    pub UnFramedBERT: B1,
}

/// Receive Interface Control Register (RICR) - 0xN122
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct RICR {
    pub RxSyncFrD: B1,
    #[skip] __: B1,
    pub RxPLClkEnb_RxSync_Is_Low: B1,
    pub RxFr1544: B1,
    pub RxICLKINV: B1,
    pub RxMUXEN: B1,
    pub RxIMODE: B2,
}

/// BERT Control and Status Register (BERTCSR1) - 0xN123
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct BERTCSR1 {
    pub PRBSTyp: B1,
    pub ERRORIns: B1,
    pub DATAInv: B1,
    pub RxBERTLock: B1,
    pub RxBERTEnb: B1,
    pub TxBERTEnb: B1,
    pub RxBypass: B1,
    pub TxBypass: B1,
}

/// Loopback Code Control Register - Code 0 (LCCR0) - 0xN124
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct LCCR0 {
    pub RXLBCALEN: B2,
    pub RXLBCDLEN: B2,
    pub TXLBCLEN: B2,
    pub FRAMED: B1,
    pub AUTOENB: B1,
}

/// Loopback Code Control Register - Code 1 (LCCR1) - 0xN12a
/// Loopback Code Control Register - Code 2 (LCCR2) - 0xN12d
/// Loopback Code Control Register - Code 3 (LCCR3) - 0xN146
/// Loopback Code Control Register - Code 4 (LCCR4) - 0xN149
/// Loopback Code Control Register - Code 5 (LCCR5) - 0xN14c
/// Loopback Code Control Register - Code 6 (LCCR6) - 0xN14f
/// Loopback Code Control Register - Code 7 (LCCR7) - 0xN156
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct LCCR {
    pub RXLBCALEN: B2,
    pub RXLBCDLEN: B2,
    #[skip] __: B2,
    pub FRAMED: B1,
    #[skip] __: B1,
}

/// Transmit Loopback Code Register (TLCR) - 0xN125
///
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct TLCR {
    pub TXLBC: B7,
    pub TXLBCENB: B1,
}

/// Receive Loopback Activation Code Register - Code 0  (RLACR0) - 0xN126
/// Receive Loopback Activation Code Register - Code 1  (RLACR1) - 0xN12b
/// Receive Loopback Activation Code Register - Code 2  (RLACR2) - 0xN12e
/// Receive Loopback Activation Code Register - Code 3  (RLACR3) - 0xN147
/// Receive Loopback Activation Code Register - Code 4  (RLACR4) - 0xN14a
/// Receive Loopback Activation Code Register - Code 5  (RLACR5) - 0xN14d
/// Receive Loopback Activation Code Register - Code 6  (RLACR6) - 0xN150
/// Receive Loopback Activation Code Register - Code 7  (RLACR7) - 0xN157
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct RLACR {
    pub RXLBAC: B7,
    pub RXLBACENB: B1,
}

/// Receive Loopback Deactivation Code Register - Code 0 (RLDCR0) - 0xN127
/// Receive Loopback Deactivation Code Register - Code 1 (RLDCR1) - 0xN12c
/// Receive Loopback Deactivation Code Register - Code 2 (RLDCR2) - 0xN12f
/// Receive Loopback Deactivation Code Register - Code 3 (RLDCR3) - 0xN148
/// Receive Loopback Deactivation Code Register - Code 4 (RLDCR4) - 0xN14b
/// Receive Loopback Deactivation Code Register - Code 5 (RLDCR5) - 0xN14d
/// Receive Loopback Deactivation Code Register - Code 6 (RLDCR6) - 0xN151
/// Receive Loopback Deactivation Code Register - Code 7 (RLDCR7) - 0xN158
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct RLDCR {
    pub RXLBDC: B7,
    pub RXLBDCENB: B1,
}

/// Receive LoopCode Detection Switch (RLCDS) - 0xN128
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct RLCDS {
    pub RxLCDetSwitch: u8,
}

// impl RLCDS {
//     pub fn from_bytes(b: [u8; 1]) -> Self {
//         Self {
//             RxLCDetSwitch: b[0],
//         }
//     }
// }

// impl fmt::Debug for RLCDS {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "RLCDS {{ RxLCDetSwitch: 0b{:08b} }}", self.RxLCDetSwitch)
//     }
// }

/// Defect Detection Enable Register (DDER) - 0xN129
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct DDER {
    pub DEFDET: B1,
    #[skip] __: B7,
}

/// Transmit LoopCode Generation Switch (TLCGS) - 0xN140
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct TLCGS {
    #[skip] __: B7,
    pub TxLCGenSwitch: B1,
}

/// LoopCode Timer Select (LCTS) - 0xN141
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct LCTS {
    #[skip] __: B5,
    pub LCTimer: B3,
}

/// Transmit SPRM and NPRM Control Register (TSPRMCR) - 0xN142
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct TSPRMCR {
    pub FC_Bit: B1,
    pub PA_Bit: B1,
    pub U1_Bit: B1,
    pub U2_Bit: B1,
    pub R_Bit: B4,
}

#[derive(Copy, Clone, BitfieldSpecifier, Debug)]
#[bits=4]
pub enum BERTPattern {
    PRBS_X20_X3_1 = 0b0010,
    QRSS_X20_X17_1 = 0b0011,
    AllOnes = 0b0100,
    AllZeros = 0b0101,
    ThreeIn24 = 0b0110,
    OneIn8 = 0b0111,
    Fifty5Octet = 0b1000,
    Daly = 0b1001,
    PRBS_X20_X17_1 = 0b1010,
}

/// BERT Control Register (BCR) - 0xN163
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct BCR {
    #[skip] __: B4,
    pub BERT: BERTPattern,
}

/// SSM BOC Control Register (BOCCR) - 0xN170
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct BOCCR {
    pub TxABORT: B1,
    pub RMF: B2,
    pub RBOCE: B1,
    pub BOCR: B1,
    pub RBF: B2,
    pub SBOC: B1,
}

/// SSM Receive FDL Register (RFDLR) - 0xN171
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct RFDLR {
    #[skip] __: B2,
    pub RBOC: B6,
}

/// SSM Receive FDL Match 1 Register (RFDLMR1) - 0xN172
/// SSM Receive FDL Match 1 Register (RFDLMR2) - 0xN173
/// SSM Receive FDL Match 1 Register (RFDLMR3) - 0xN174
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct RFDLMR {
    #[skip] __: B2,
    pub RFDLM: B6,
}

/// SSM Transmit FDL Register (TFDLR) - 0xN175
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct TFDLR {
    #[skip] __: B2,
    pub TBOC: B6,
}

/// SSM Transmit Byte Count Register (TBCR) - 0xN176
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct TBCR {
    pub TBCR: B8,
}

/// Receive DS-0 Monitor Register (RDS0MR) - 0xN15f through 0xN16f (not including 0xN163!)
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct RDS0MR {
    pub RxDS_0: B8,
}

/// Transmit DS-0 Monitor Register (TDS0MR) - 0xN1d0 through 0xN1ef
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct TDS0MR {
    pub TxDS_0: B8,
}

///////////////////////////////////////////////////////////////////////
/// Time SLot (payload) Control

#[derive(Copy, Clone, BitfieldSpecifier, Debug)]
#[bits=2]
pub enum TransmitLAPDSource {
    LAPDController1 = 0b00,
    LAPDController2 = 0b01,
    TSDLSR_TxDE = 0b10,
    LAPDController3 = 0b11,
}

#[derive(Copy, Clone, BitfieldSpecifier, Debug)]
#[bits=2]
pub enum ZeroCodeSuppression {
    None = 0b00,
    ATTBit7Stuffing = 0b01,
    GTE = 0b10,
    DDS = 0b11,
}

#[derive(Copy, Clone, BitfieldSpecifier, Debug)]
#[bits=4]
pub enum ChannelConditioning {
    Unchanged = 0x0,
    InvertAll = 0x1,
    InvertEven = 0x2,
    InvertOdd = 0x3,
    PUCR = 0x4,
    BUSY = 0x5,
    VACANT = 0x6,
    BUSYWithSlotNumber = 0x7,
    MOOF = 0x8,
    ALawMilliwatt = 0x9,
    ULawMilliwatt = 0xa,
    InvertMSB = 0xb,
    InvertAllButMSB = 0xc,
    PRBSOrQRTS = 0xd,
    DETimeSlot = 0xf,
}

/// Transmit Channel Control Registers 0-23 (TCCR) - 0xN300 - 0xN317
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct TCCR {
    pub LAPDcntl: TransmitLAPDSource,
    pub TxZERO: ZeroCodeSuppression,
    pub TxCOND: ChannelConditioning,
}

/// Transmit User Code Register 0-23 (TUCR) - 0xN320 - 0xN337
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct TUCR {
    pub TUCR: B8,
}

#[derive(Copy, Clone, BitfieldSpecifier, Debug)]
#[bits=2]
pub enum ChannelSignalingSource {
    PCMData = 0b00,
    TSCR = 0b01,
    TxSIG = 0b10,
}

/// Transmit Signaling Control Register 0-23 (TSCR) - 0xN340 - 0xN357
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct TSCR {
    pub A_x: B1,
    pub B_y: B1,
    pub C_x: B1,
    pub D_x: B1,
    #[skip] __: B1,
    pub Rob_Enb: B1,
    pub TxSIGSRC: ChannelSignalingSource,
}

#[derive(Copy, Clone, BitfieldSpecifier, Debug)]
#[bits=2]
pub enum ReceiveLAPDSource {
    LAPDController1 = 0b00,
    LAPDController2 = 0b01,
    RSDLSR_RxDE = 0b10,
    LAPDController3 = 0b11,
}

/// Receive Channel Control Registers 0-23 (RCCR) - 0xN360 - 0xN377
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct RCCR {
    pub LAPDcntl: ReceiveLAPDSource,
    pub RxZERO: ZeroCodeSuppression,
    pub RxCOND: ChannelConditioning,
}

/// Receive User Code Register 0-23 (RUCR) - 0xN380 - 0xN397
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct RUCR {
    pub RxUSER: B8,
}

#[derive(Copy, Clone, BitfieldSpecifier, Debug)]
#[bits=2]
pub enum ReceiveSignalingConditioning {
    AllOnes = 0b00,
    SixteenCode_ABCD = 0b01,
    FourCode_AB = 0b10,
    TwoCode_A = 0b11,
}

#[derive(Copy, Clone, BitfieldSpecifier, Debug)]
#[bits=2]
pub enum ReceiveSignalingExtraction {
    None = 0b00,
    SixteenCode_ABCD = 0b01,
    FourCode_AB = 0b10,
    TwoCode_A = 0b11,
}

/// Receive Signaling Control Register 0-23 (RSCR) - 0xN3a0 - 0xN3b7
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct RSCtR {
    #[skip] __: B1,
    pub SIGC_ENB: B1,
    pub OH_ENB: B1,
    pub DEB_ENB: B1,
    pub RxSIGC: ReceiveSignalingConditioning,
    pub RxSIGE: ReceiveSignalingExtraction,
}

/// Receive Substitution Signaling Register 0-23 (RSSR) - 0xN3c0 - 0xN3d7
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct RSSR {
    #[skip] __: B4,
    pub SIG_16A_4A_2A: B1,
    pub SIG_16B_4B_2A: B1,
    pub SIG_16C_4A_2A: B1,
    pub SIG_16D_4B_2A: B1,
}

/// Receive Signaling Array Register 0-23 (RSAR) - 0xN500 - 0xN517
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct RSAR {
    #[skip] __: B4,
    pub A: B1,
    pub B: B1,
    pub C: B1,
    pub D: B1,
}

///////////////////////////////////////////////////////////////////////
// LAPD Buffers, 0xN600 - 0xN65f, 0xN700 - 0xN75f

pub type LAPDBCR = u8;

///////////////////////////////////////////////////////////////////////
// Performance Monitors (PMON), 0xN900 - 0xN92c

pub type RLCVCU = u8;
pub type RLCVCL = u8;

pub type RFAECU = u8;
pub type RFAECL = u8;

pub type RSEFC = u8;

pub type RSBBECU = u8;
pub type RSBBECL = u8;

pub type RSC = u8;

pub type RLFC = u8;

pub type RCFAC = u8;

pub type LFCSEC1 = u8;

pub type PBECU = u8;
pub type PBECL = u8;

pub type TSC = u8;

pub type EZVCU = u8;
pub type EZVCL = u8;

pub type LFCSEC2 = u8;

pub type LFCSEC3 = u8;

///////////////////////////////////////////////////////////////////////
// Interrupts and Status

/// Block Interrupt Status Register (BISR) - 0xNb00
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct BISR {
    #[skip] __: B1,
    pub LBCODE: B1,
    pub RxClkLOS: B1,
    pub ONESEC: B1,
    pub HDLC: B1,
    pub SLIP: B1,
    pub ALARM: B1,
    pub T1FRAME: B1,
}

/// Block Interrupt Enable Register (BIER) - 0xNb01
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct BIER {
    #[skip] __: B1,
    pub LBCODE_ENB: B1,
    pub RXCLKLOSS: B1,
    pub ONESEC_ENB: B1,
    pub HDLC_ENB: B1,
    pub SLIP_ENB: B1,
    pub ALARM_ENB: B1,
    pub T1FRAME_ENB: B1,
}

/// Alarm and Error Interrupt Status Register (AEISR) - 0xNb02
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct AEISR {
    pub RxOOF_State: B1,
    pub RxAIS_State: B1,
    pub RxYEL_State: B1,
    pub LOS_State: B1,
    pub LCVInt_Status: B1,
    pub RxOOF_State_Change: B1,
    pub RxAIS_State_Change: B1,
    pub RxYEL_State_Change: B1,
}

/// Alarm and Error Interrupt Enable Register (AEIER) - 0xNb03
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct AEIER {
    #[skip] __: B3,
    pub SetToZero: B1,
    pub LCV_ENB: B1,
    pub RxOOF_ENB: B1,
    pub RxAIS_ENB: B1,
    pub RxYEL_ENB: B1,
}

/// Framer Interrupt Status Register (FISR) - 0xNb04
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct FISR {
    pub DS0_Change: B1,
    pub DS0_Status: B1,
    pub SIG: B1,
    pub COFA: B1,
    pub OOF_Status: B1,
    pub FMD: B1,
    pub SE: B1,
    pub FE: B1,
}

/// Framer Interrupt Enable Register (FIER) - 0xNb05
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct FIER {
    pub DS0_ENB: B1,
    #[skip] __: B1,
    pub SIG_ENB: B1,
    pub COFA_ENB: B1,
    pub OOF_ENB: B1,
    pub FMD_ENB: B1,
    pub SE_ENB: B1,
    pub FE_ENB: B1,
}

/// Data Link Status Register 1 (DLSR1) - 0xNb06
/// Data Link Status Register 2 (DLSR2) - 0xNb16
/// Data Link Status Register 3 (DLSR3) - 0xNb26
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct DLSRx {
    pub MSG_TYPE: B1,
    pub TxSOT: B1,
    pub RxSOT: B1,
    pub TxEOT: B1,
    pub RxEOT: B1,
    pub FCS_ERR: B1,
    pub RxABORT: B1,
    pub RxIDLE: B1,
}

/// Data Link Interrupt Enable Register 1 (DLIER1) - 0xNb07
/// Data Link Interrupt Enable Register 2 (DLIER2) - 0xNb17
/// Data Link Interrupt Enable Register 3 (DLIER3) - 0xNb27
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct DLIERx {
    #[skip] __: B1,
    pub TxSOT_ENB: B1,
    pub RxSOT_ENB: B1,
    pub TxEOT_ENB: B1,
    pub RxEOT_ENB: B1,
    pub FCS_ERR_ENB: B1,
    pub RxABORT_ENB: B1,
    pub RxIDLE_ENB: B1,
}

/// Slip Buffer Interrupt Status Register (SBISR) - 0xNb08
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct SBISR {
    pub TxSB_FULL: B1,
    pub TxSB_EMPT: B1,
    pub TxSB_SLIP: B1,
    pub SLC96_LOCK: B1,
    pub Multiframe_LOCK: B1,
    pub RxSB_FULL: B1,
    pub RxSB_EMPT: B1,
    pub RxSB_SLIP: B1,
}

/// Slip Buffer Interrupt Enable Register (SBIER) - 0xNb09
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct SBIER {
    pub TxFULL_ENB: B1,
    pub TxEMPT_ENB: B1,
    pub TxSLIP_ENB: B1,
    #[skip] __: B2,
    pub RxFULL_ENB: B1,
    pub RxEMPT_ENB: B1,
    pub RxSLIP_ENB: B1,
}

/// Receive Loopback Code 0 Interrupt and Status Register (RLCISR0) - 0xNb0a
/// Receive Loopback Code 1 Interrupt and Status Register (RLCISR1) - 0xNb14
/// Receive Loopback Code 2 Interrupt and Status Register (RLCISR2) - 0xNb1a
/// Receive Loopback Code 3 Interrupt and Status Register (RLCISR3) - 0xNb1c
/// Receive Loopback Code 4 Interrupt and Status Register (RLCISR4) - 0xNb1e
/// Receive Loopback Code 5 Interrupt and Status Register (RLCISR5) - 0xNb20
/// Receive Loopback Code 6 Interrupt and Status Register (RLCISR6) - 0xNb22
/// Receive Loopback Code 7 Interrupt and Status Register (RLCISR7) - 0xNb24
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct RLCISRx {
    #[skip] __: B4,
    pub RXASTAT: B1,
    pub RXDSTAT: B1,
    pub RXAINT: B1,
    pub RXDINT: B1,
}

/// Receive Loopback Code 0 Interrupt Enable Register (RLCIER0) - 0xNb0b
/// Receive Loopback Code 1 Interrupt Enable Register (RLCIER1) - 0xNb15
/// Receive Loopback Code 2 Interrupt Enable Register (RLCIER2) - 0xNb1b
/// Receive Loopback Code 3 Interrupt Enable Register (RLCIER3) - 0xNb1d
/// Receive Loopback Code 4 Interrupt Enable Register (RLCIER4) - 0xNb1f
/// Receive Loopback Code 5 Interrupt Enable Register (RLCIER5) - 0xNb21
/// Receive Loopback Code 6 Interrupt Enable Register (RLCIER6) - 0xNb23
/// Receive Loopback Code 7 Interrupt Enable Register (RLCIER7) - 0xNb25
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct RLCIERx {
    #[skip] __: B6,
    pub RXAENB: B1,
    pub RXDENB: B1,
}

/// Excessive Zero Status Register (EXZSR) - 0xNb0e
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct EXZSR {
    #[skip] __: B7,
    pub EXZ_STATUS: B1,
}

/// Excessive Zero Enable Register (EXZER) - 0xNb0f
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct EXZER {
    #[skip] __: B7,
    pub EXZ_ENB: B1,
}

/// SS7 Status Register for LAPD1 (SS7SR1) - 0xNb10
/// SS7 Status Register for LAPD2 (SS7SR2) - 0xNb18
/// SS7 Status Register for LAPD3 (SS7SR3) - 0xNb28
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct SS7SRx {
    #[skip] __: B7,
    pub SS7_STATUS: B1,
}

/// SS7 Enable Register for LAPD1 (SS7ER1) - 0xNb11
/// SS7 Enable Register for LAPD2 (SS7ER2) - 0xNb19
/// SS7 Enable Register for LAPD3 (SS7ER3) - 0xNb29
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct SS7ERx {
    #[skip] __: B7,
    pub SS7_ENB: B1,
}

/// RxLOS/CRC Interrupt Status Register (RLCISR) - 0xNb12
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct RLCISR {
    #[skip] __: B4,
    pub RxLOSINT: B1,
    #[skip] __: B3,
}

/// RxLOS/CRC Interrupt Enable Register (RLCIER) - 0xNb13
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct RLCIER {
    #[skip] __: B4,
    pub RxLOS_ENB: B1,
    #[skip] __: B3,
}

/// Customer Installation Alarm Status Register (CIASR) - 0xNb40
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct CIASR {
    #[skip] __: B2,
    pub RxAIS_CI_state: B1,
    pub RxRAI_CI_state: B1,
    #[skip] __: B2,
    pub RxAIS_CI: B1,
    pub RxRAI_CI: B1,
}

/// Customer Installation Alarm Enable Register (CIAIER) - 0xNb41
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct CIAIER {
    #[skip] __: B6,
    pub RxAIS_CI_ENB: B1,
    pub RxRAI_CI_ENB: B1,
}

/// T1 BOC Interrupt Status Register (BOCISR) - 0xNb70
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct BOCISR {
    pub RMTCH3: B1,
    pub RMTCH2: B1,
    pub BOCC: B1,
    pub RFDLAD: B1,
    pub RFDLF: B1,
    pub TFDLE: B1,
    pub RMTCH1: B1,
    pub RBOC: B1,
}

/// T1 BOC Interrupt Enable Register (BOCIER) - 0xNb71
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct BOCIER {
    pub RMTCH3: B1,
    pub RMTCH2: B1,
    pub BOCC: B1,
    pub RFDLAD: B1,
    pub RFDLF: B1,
    pub TFDLE: B1,
    pub RMTCH1: B1,
    pub RBOC: B1,
}

/// T1 BOC Unstable Interrupt Status Register (BOCUISR) - 0xNb74
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct BOCUISR {
    #[skip] __: B1,
    pub Unstable: B1,
    #[skip] __: B6,
}

/// T1 BOC Unstable Interrupt Enable Register (BOCUIER) - 0xNb75
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct BOCUIER {
    #[skip] __: B1,
    pub Unstable: B1,
    #[skip] __: B6,
}

///////////////////////////////////////////////////////////////////////
// LIU Channel Control, 0x0fN0 - 0x0fNf

#[derive(Copy, Clone, BitfieldSpecifier, Debug)]
#[bits=1]
pub enum PRBSPattern {
    PRBS = 0b0,
    QRSS = 0b1,
}

#[derive(Copy, Clone, BitfieldSpecifier, Debug)]
#[bits=1]
pub enum PRBSDestination {
    TTIP_TRING = 0b0,
    RPOS_RCLK = 0b1,
}

/// LIU Channel Control Register 0 (LIUCCR0) - 0x0fN0
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct LIUCCR0 {
    pub QRSS_n_PRBS_n: PRBSPattern,
    pub PRBS_Rx_n_PRBS_Tx_n: PRBSDestination,
    pub RXON_n: B1,
    pub EQC: B5,
}

#[derive(Copy, Clone, BitfieldSpecifier, Debug)]
#[bits=1]
pub enum Termination {
    HighImpedance = 0b0,
    Internal = 0b1,
}

#[derive(Copy, Clone, BitfieldSpecifier, Debug)]
#[bits=2]
pub enum TerminationImpedance {
    Ohms100 = 0b00,
    Ohms110 = 0b01,
    Ohms75 = 0b10,
    Ohms120 = 0b11,
}

/// LIU Channel Control Register 1 (LIUCCR1) - 0x0fN1
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct LIUCCR1 {
    pub RXTSEL_n: Termination,
    pub TXTSEL_n: Termination,
    pub TERSEL: TerminationImpedance,
    pub RxJASEL_n: B1,
    pub TxJASEL_n: B1,
    pub JABW_n: B1,
    pub FIFOS_n: B1,
}

#[derive(Copy, Clone, BitfieldSpecifier, Debug)]
#[bits=3]
pub enum TransmitTestPattern {
    None = 0b000,
    TDQRSS = 0b100,
    TAOS = 0b101,
    TLUC = 0b110,
    TLDC = 0b111,
}

#[derive(Copy, Clone, BitfieldSpecifier, Debug)]
#[bits=3]
pub enum LIULoopback {
    None = 0b000,
    Dual = 0b100,
    Analog = 0b101,
    Remote = 0b110,
    Digital = 0b111,
}

/// LIU Channel Control Register 2 (LIUCCR2) - 0x0fN2
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct LIUCCR2 {
    pub INVQRSS_n: B1,
    pub TXTEST: TransmitTestPattern,
    pub TXON_n: B1,
    pub LOOP2_n: LIULoopback,
}

#[derive(Copy, Clone, BitfieldSpecifier, Debug)]
#[bits=2]
pub enum NetworkLoopCodeDetection {
    Disabled = 0b00,
    LoopUpDetection = 0b01,
    LoopDownDetection = 0b10,
    AutomaticLoopUpDetectionAndRemoteActivation = 0b11,
}

/// LIU Channel Control Register 3 (LIUCCR3) - 0x0fN3
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct LIUCCR3 {
    pub NLCDE: NetworkLoopCodeDetection,
    pub CODES_n: B1,
    #[skip] __: B2,
    pub INSBPV_n: B1,
    pub INSBER_n: B1,
    #[skip] __: B1,
}

/// LIU Channel Control Interrupt Enable Register 3 (LIUCCIER) - 0x0fN4
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct LIUCCIER {
    #[skip] __: B1,
    pub DMOIE_n: B1,
    pub FLSIE_n: B1,
    pub LCVIE_n: B1,
    pub NLCDIE_n: B1,
    pub AISDIE_n: B1,
    pub RLOSIE_n: B1,
    pub QRPDIE_n: B1,
}

/// LIU Channel Control Status Register 3 (LIUCCSR) - 0x0fN5
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct LIUCCSR {
    #[skip] __: B1,
    pub DMO_n: B1,
    pub FLS_n: B1,
    pub LCV_n: B1,
    pub NLCD_n: B1,
    pub AISD_n: B1,
    pub RLOS_n: B1,
    pub QRPD_n: B1,
}

/// LIU Channel Control Interrupt Status Register 3 (LIUCCISR) - 0x0fN6
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct LIUCCISR {
    #[skip] __: B1,
    pub DMOIS_n: B1,
    pub FLSIS_n: B1,
    pub LCVIS_n: B1,
    pub NLCDIS_n: B1,
    pub AISDIS_n: B1,
    pub RLOSIS_n: B1,
    pub QRPDIS_n: B1,
}

/// LIU Channel Control Cable Loss Register 3 (LIUCCCCR) - 0x0fN7
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct LIUCCCCR {
    #[skip] __: B2,
    pub CLOS: B6,
}

/// LIU Channel Control Arbitrary Register 3 (LIUCCAR) - 0x0fN8 - 0x0fNf
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct LIUCCAR {
    #[skip] __: B1,
    pub Arb_Seg: B7,
}

///////////////////////////////////////////////////////////////////////
// LIU Global Control, 0x0fe0 - 0x0fea

/// LIU Global Control Register 0 (LIUGCR0) - 0x0fe0
///
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct LIUGCR0 {
    pub SR: B1,
    pub ATAOS: B1,
    pub RCLKE: B1,
    pub TCLKE: B1,
    pub DATAP: B1,
    #[skip] __: B1,
    pub GIE: B1,
    pub SRESET: B1,
}

#[derive(Copy, Clone, BitfieldSpecifier, Debug)]
#[bits=2]
pub enum WireGauge {
    Gauge22And24 = 0b00,
    Gauge22 = 0b01,
    Gauge24 = 0b10,
    Gauge26 = 0b11,
}

/// LIU Global Control Register 1 (LIUGCR1) - 0x0fe1
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct LIUGCR1 {
    pub TxSYNC_Sect13: B1,
    pub RxSYNC_Sect13: B1,
    pub Gauge: WireGauge,
    #[skip] __: B1,
    pub RXMUTE: B1,
    pub EXLOS: B1,
    pub ICT: B1,
}

/// LIU Global Control Register 2 (LIUGCR2) - 0x0fe2
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct LIUGCR2 {
    pub Force_to_0: B1,
    #[skip] __: B7,
}

/// LIU Global Control Register 3 (LIUGCR3) - 0x0fe4
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct LIUGCR3 {
    #[skip] __: B8,
}

#[derive(Copy, Clone, BitfieldSpecifier, Debug)]
#[bits=4]
pub enum ClockSelect {
    M2_048 = 0b0000,
    M1_544 = 0b0001,
    M4_096 = 0b1000,
    M3_088 = 0b1001,
    M8_192 = 0b1010,
    M6_176 = 0b1011,
    M16_384 = 0b1100,
    M12_352 = 0b1101,
}

/// LIU Global Control Register 4 (LIUGCR4) - 0x0fe9
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct LIUGCR4 {
    #[skip] __: B4,
    pub CLKSEL: ClockSelect,
}

/// LIU Global Control Register 5 (LIUGCR5) - 0x0fea
/// 
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct LIUGCR5 {
    #[skip] __: B7,
    pub GCHIS0: B1,
}

///////////////////////////////////////////////////////////////////////
/// 

/// Device ID Register (DEVID) - 0x1fe
///
/// This register is used to identify the XRT86VX38 Framer/LIU. The
/// value of this register is 0x3Ch.
///
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct DEVID {
    pub DEVID: B8,
}

/// Revision ID Register (REVID) - 0x1ff
/// 
/// This register is used to identify the revision number of the XRT86VX38.
/// The value of this register for the first revision is A - 0x01h.
/// NOTE : The content of this register is subject to change when a newer
/// revision of the device is issued.
#[bitfield(bits=8)]
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub struct REVID {
    pub REVID: B8,
}

///////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use crate::framer::register::{CSR, ClockSource};

    #[test]
    fn modular_bitfield_is_msb_first() {
        let dut = CSR::new()
            .with_LCV_Insert(1)
            .with_Set_T1_Mode(0)
            .with_Sync_All_Transmitters_to_8kHz(1)
            .with_Clock_Loss_Detect(0)
            .with_CSS(ClockSource::Loop)
            ;

        assert_eq!(dut.into_bytes(), [0xa0]);
    }    
}
