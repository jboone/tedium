use std::{io::{Read, Cursor, self}, slice, sync::{Arc, Mutex}};

use crossbeam::channel::Sender;

use console::{style, Color};
use rusb::{ffi, constants::*, UsbContext};

use crate::framer::{register::*, device::{open_device}, usb::{EndpointNumber, InterfaceNumber, Transfer, CallbackWrapper, from_libusb}};

use super::{FramerEvent, usb::{TransferHandler, INTERRUPT_BYTES_MAX}, device};

pub struct LoopbackCodeStatus {
    pub rlcisrs: [RLCISRx; 8],
}

pub struct HDLCControllerStatus {
    pub dlsr: DLSRx,
    pub rdlbcr: RDLBCR,
    pub data: Vec<u8>,
    pub ss7sr: SS7SRx,
}

pub struct HDLCStatus {
    pub controller: [HDLCControllerStatus; 3],
}

pub struct SlipStatus {
    pub sbisr: SBISR,
}

pub struct AlarmStatus {
    pub aeisr: AEISR,
    pub exzsr: EXZSR,
    pub ciasr: CIASR,
}

pub struct T1FrameStatus {
    pub fisr: FISR,
    pub sig: Option<ReceiveSignalingStatus>,
}

pub struct ReceiveSignalingStatus {
    pub rsars: [RSAR; 24],
}

pub struct FramerInterruptStatus {
    pub channel_index: usize,
    pub bisr: BISR,
    pub lbcode: Option<LoopbackCodeStatus>,
    pub hdlc: Option<HDLCStatus>,
    pub slip: Option<SlipStatus>,
    pub alarm: Option<AlarmStatus>,
    pub t1frame: Option<T1FrameStatus>,
}

///////////////////////////////////////////////////////////////////////

impl FramerInterruptStatus {
    pub fn from_slice(b: &[u8]) -> io::Result<Self> {
        let mut r = Cursor::new(b);
        let result = Self::from_read(&mut r);
        assert_eq!(b.len() as u64, r.position());
        result
    }

    fn from_read(r: &mut impl Read) -> io::Result<Self> {
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
    fn from_read(r: &mut impl Read) -> io::Result<Self> {
        let mut rlcisrs = [0u8; 8];
        r.read(&mut rlcisrs)?;
        let rlcisrs = rlcisrs.map(|v| RLCISRx::from(v));

        Ok(Self {
            rlcisrs,
        })
    }
}

impl HDLCStatus {
    fn from_read(r: &mut impl Read) -> io::Result<Self> {
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
    fn from_read(r: &mut impl Read) -> io::Result<Self> {
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
    fn from_read(r: &mut impl Read) -> io::Result<Self> {
        let mut sbisr = [0u8; 1];
        r.read(&mut sbisr)?;
        let sbisr = SBISR::from(sbisr[0]);

        Ok(Self {
            sbisr,
        })
    }
}

impl AlarmStatus {
    fn from_read(r: &mut impl Read) -> io::Result<Self> {
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
    fn from_read(r: &mut impl Read) -> io::Result<Self> {
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
    fn from_read(r: &mut impl Read) -> io::Result<Self> {
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

struct FramerInterruptHandler {
    sender: Sender<FramerEvent>,
}

impl FramerInterruptHandler {
    fn new(sender: Sender<FramerEvent>) -> Self {
        Self {
            sender,
        }
    }
}

impl TransferHandler for FramerInterruptHandler {
    fn callback(&self, transfer: *mut ffi::libusb_transfer) {
        let status = unsafe { (*transfer).status };
        let actual_length = unsafe { (*transfer).actual_length };

        let result = unsafe {
            ffi::libusb_submit_transfer(transfer)
        };
        match result {
            LIBUSB_SUCCESS => {},
            e => eprintln!("IN: libusb_submit_transfer error: {e}"),
        }

        // TODO: This gets called back even when the device endpoint
        // reports a zero-length packet. Is there a better way, one which
        // doesn't trouble the host with 1,000 packets a second, the vast
        // majority of which require no work on the host's part?
        if status == LIBUSB_TRANSFER_COMPLETED && actual_length > 0 {
            let actual_length = actual_length.try_into().unwrap();
            let mut data = [0u8; INTERRUPT_BYTES_MAX];
            
            // TODO: Replace this and so much other transfer-related code
            // with "safe" functions on the Transfer struct.
            let buffer = unsafe {
                let buffer = (*transfer).buffer;
                slice::from_raw_parts_mut(buffer, actual_length)
            };

            data[0..actual_length].copy_from_slice(buffer);
            let message = FramerEvent::Interrupt(data, actual_length);
            if let Err(e) = self.sender.send(message) {
                eprint!("error: data.sender.send: {:?}", e);
            }
        }
    }
}

pub struct FramerInterruptThread {
    
}

impl FramerInterruptThread {
    pub fn run(sender: Sender<FramerEvent>) -> device::Result<()> {
        let mut context = rusb::Context::new()?;

        let mut device = open_device(&mut context)?;

        let endpoint = LIBUSB_ENDPOINT_IN | EndpointNumber::Interrupt as u8;

        device.claim_interface(InterfaceNumber::Interrupt as u8)?;
        device.set_alternate_setting(InterfaceNumber::Interrupt as u8, 0)?;

        let device = Arc::new(device);

        let device_handle = &device;

        const TRANSFERS_COUNT: usize = 4;

        let mut transfers: Vec<Transfer> = Vec::new();
        
        let handler = Arc::new(Mutex::new(FramerInterruptHandler::new(sender)));
        
        for _ in 0..TRANSFERS_COUNT {
            let transfer = Transfer::new_interrupt_transfer(
                device_handle.clone(),
                endpoint,
                INTERRUPT_BYTES_MAX,
                0,
                Box::new(CallbackWrapper::new(handler.clone())),
            );

            transfer.submit();
            transfers.push(transfer);
        }

        let context = device.context();

        loop {
            let result = unsafe {
                ffi::libusb_handle_events(context.as_raw())
            };
            if result != 0 {
                eprintln!("error: libusb_handle_events: {:?}", result);
                // TODO: I wish I could return an official rusb::Error here,
                // but the function that turns a raw i32 result into an Error
                // is private to the rusb crate.
                return Err(from_libusb(result));
            }
        }
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
