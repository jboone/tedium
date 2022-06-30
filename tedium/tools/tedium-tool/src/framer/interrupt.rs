use std::io::{Read, Cursor, Result};

use console::{style, Color};

use crate::framer::register::*;

pub struct LoopbackCodeStatus {
    rlcisrs: [RLCISRx; 8],
}

pub struct HDLCControllerStatus {
    dlsr: DLSRx,
    rdlbcr: RDLBCR,
    data: Vec<u8>,
    ss7sr: SS7SRx,
}

pub struct HDLCStatus {
    controller: [HDLCControllerStatus; 3],
}

pub struct SlipStatus {
    sbisr: SBISR,
}

pub struct AlarmStatus {
    aeisr: AEISR,
    exzsr: EXZSR,
    ciasr: CIASR,
}

pub struct T1FrameStatus {
    fisr: FISR,
    sig: Option<ReceiveSignalingStatus>,
}

pub struct ReceiveSignalingStatus {
    rsars: [RSAR; 24],
}

pub struct FramerInterruptStatus {
    channel_index: usize,
    bisr: BISR,
    lbcode: Option<LoopbackCodeStatus>,
    hdlc: Option<HDLCStatus>,
    slip: Option<SlipStatus>,
    alarm: Option<AlarmStatus>,
    t1frame: Option<T1FrameStatus>,
}

///////////////////////////////////////////////////////////////////////

impl FramerInterruptStatus {
    pub fn from_slice(b: &[u8]) -> Result<Self> {
        let mut r = Cursor::new(b);
        let result = Self::from_read(&mut r);
        assert_eq!(b.len() as u64, r.position());
        result
    }

    fn from_read(r: &mut impl Read) -> Result<Self> {
        let mut channel_index = [0u8; 1];
        r.read(&mut channel_index)?;
        let channel_index = channel_index[0] as usize;

        let mut bisr = [0u8; 1];
        r.read(&mut bisr)?;
        let bisr = BISR::from(bisr[0]);

        let lbcode = if bisr.LBCODE() != 0 {
            Some(LoopbackCodeStatus::from_read(r)?)
        } else {
            None
        };

        if bisr.RxClkLOS() != 0 {

        }
    
        if bisr.ONESEC() != 0 {
    
        }
    
        let hdlc = if bisr.HDLC() != 0 {
            Some(HDLCStatus::from_read(r)?)
        } else {
            None
        };

        let slip = if bisr.SLIP() != 0 {
            Some(SlipStatus::from_read(r)?)
        } else {
            None
        };

        let alarm = if bisr.ALARM() != 0 {
            Some(AlarmStatus::from_read(r)?)
        } else {
            None
        };

        let t1frame = if bisr.T1FRAME() != 0 {
            Some(T1FrameStatus::from_read(r)?)
        } else {
            None
        };

        Ok(Self {
            channel_index,
            bisr,
            lbcode,
            hdlc,
            slip,
            alarm,
            t1frame,
        })
    }
}

impl LoopbackCodeStatus {
    fn from_read(r: &mut impl Read) -> Result<Self> {
        let mut rlcisrs = [0u8; 8];
        r.read(&mut rlcisrs)?;
        let rlcisrs = rlcisrs.map(|v| RLCISRx::from(v));

        Ok(Self {
            rlcisrs,
        })
    }
}

impl HDLCStatus {
    fn from_read(r: &mut impl Read) -> Result<Self> {
        Ok(Self {
            controller: [
                HDLCControllerStatus::from_read(r)?,
                HDLCControllerStatus::from_read(r)?,
                HDLCControllerStatus::from_read(r)?,
            ],
        })
    }
}

impl HDLCControllerStatus {
    fn from_read(r: &mut impl Read) -> Result<Self> {
        let mut dlsr = [0u8; 1];
        r.read(&mut dlsr)?;
        let dlsr = DLSRx::from(dlsr[0]);

        let mut rdlbcr = [0u8; 1];
        r.read(&mut rdlbcr)?;
        let rdlbcr = RDLBCR::from(rdlbcr[0]);

        let rdlbc = rdlbcr.RDLBC().try_into().unwrap();
        let mut data = Vec::with_capacity(rdlbc);
        data.resize(rdlbc, 0);
        r.read(data.as_mut_slice())?;

        let mut ss7sr = [0u8; 1];
        r.read(&mut ss7sr)?;
        let ss7sr = SS7SRx::from(ss7sr[0]);

        Ok(Self {
            dlsr,
            rdlbcr,
            data,
            ss7sr,
        })
    }
}

impl SlipStatus {
    fn from_read(r: &mut impl Read) -> Result<Self> {
        let mut sbisr = [0u8; 1];
        r.read(&mut sbisr)?;
        let sbisr = SBISR::from(sbisr[0]);

        Ok(Self {
            sbisr,
        })
    }
}

impl AlarmStatus {
    fn from_read(r: &mut impl Read) -> Result<Self> {
        let mut buffer = [0u8; 3];
        r.read(&mut buffer)?;
        
        let aeisr = AEISR::from(buffer[0]);
        let exzsr = EXZSR::from(buffer[1]);
        let ciasr = CIASR::from(buffer[2]);

        Ok(Self{
            aeisr,
            exzsr,
            ciasr,
        })
    }
}

impl T1FrameStatus {
    fn from_read(r: &mut impl Read) -> Result<Self> {
        let mut fisr = [0u8; 1];
        r.read(&mut fisr)?;
        let fisr = FISR::from(fisr[0]);

        let sig = if fisr.SIG() != 0 {
            Some(ReceiveSignalingStatus::from_read(r)?)
        } else {
            None
        };

        Ok(Self {
            fisr,
            sig,
        })
    }
}

impl ReceiveSignalingStatus {
    fn from_read(r: &mut impl Read) -> Result<Self> {
        let mut buffer = [0u8; 12];
        r.read(&mut buffer)?;

        let mut rsars = [RSAR::new(); 24];
        for (i, &v) in buffer.iter().enumerate() {
            let even = RSAR::from(v >> 4);
            let odd = RSAR::from(v & 15);
            rsars[i * 2 + 0] = even;
            rsars[i * 2 + 1] = odd;
        }

        Ok(Self {
            rsars,
        })
    }
}

///////////////////////////////////////////////////////////////////////

pub fn print_framer_interrupt_status(s: &FramerInterruptStatus) {
    let color = |v| if v != 0 { Color::Red } else { Color::Green };

    let bisr = s.bisr;
    eprint!("CH{} BISR:[{}][{}][{}][{}][{}][{}][{}]",
        s.channel_index,
        style("LBCODE").fg(color(bisr.LBCODE())),
        style("RXCLOS").fg(color(bisr.RxClkLOS())),
        style("ONESEC").fg(color(bisr.ONESEC())),
        style("HDLC").fg(color(bisr.HDLC())),
        style("SLIP").fg(color(bisr.SLIP())),
        style("ALRM").fg(color(bisr.ALARM())),
        style("T1FRM").fg(color(bisr.T1FRAME())),
    );

    if let Some(lbcode) = &s.lbcode {
        eprint!(" RLCISR=[");
        for rlcisr in lbcode.rlcisrs {
            let rlcisr_u8: u8 = rlcisr.into();
            eprint!(" {rlcisr_u8:02x}");
        }
        eprint!("]");
    }

    if let Some(hdlc) = &s.hdlc {
        for (hdlc_index, controller) in hdlc.controller.iter().enumerate() {
            let dlsr = controller.dlsr;
            let dlsr_u8: u8 = dlsr.into();

            if dlsr_u8 != 0 {
                print!(" DLSR{}:[{}][{}][{}][{}][{}][{}][{}][{}]",
                    hdlc_index,
                    style("MOS").fg(color(dlsr.MSG_TYPE())),
                    style("TxSOT").fg(color(dlsr.TxSOT())),
                    style("RxSOT").fg(color(dlsr.RxSOT())),
                    style("TxEOT").fg(color(dlsr.TxEOT())),
                    style("RxEOT").fg(color(dlsr.RxEOT())),
                    style("FCS").fg(color(dlsr.FCS_ERR())),
                    style("RxABT").fg(color(dlsr.RxABORT())),
                    style("RxIDL").fg(color(dlsr.RxIDLE())),
                );

                let rdlbcr = controller.rdlbcr;
                print!(" LAPDBCR{}:[", rdlbcr.RBUFPTR());
                for data in &controller.data {
                    print!("{data:02x}");
                }
                print!("]");

                if dlsr.FCS_ERR() != 0 {
                    print!(" FCS_ERR");
                }
            }
        }
    }

    if let Some(slip) = &s.slip {
        let sbisr = slip.sbisr;
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

    if let Some(alarm) = &s.alarm {
        let aeisr = alarm.aeisr;
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

        let exzsr = alarm.exzsr;
        print!(" EXZSR:[{}]",
            style("EXZ").fg(color(exzsr.EXZ_STATUS())),
        );

        let ciasr = alarm.ciasr;
        print!(" CIASR:[{}][{}]",
            style("RAISCI").fg(color(ciasr.RxAIS_CI_state())),
            style("RRAICI").fg(color(ciasr.RxRAI_CI_state())),
        );
    }

    if let Some(t1frame) = &s.t1frame {
        let fisr = t1frame.fisr;
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

        if let Some(sig) = &t1frame.sig {
            print!(" RSAR:[");
            for rsar in sig.rsars {
                let rsar_u8: u8 = rsar.into();
                print!("{rsar_u8:1x}");
            }
            print!("]");
        }
    }

    println!();
}
