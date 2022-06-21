#![allow(non_snake_case)]

use std::{time::Duration, marker::PhantomData, os::raw::c_int, fmt};

use crossbeam::channel::Sender;
use rusb::{self, UsbContext, constants::{LIBUSB_ENDPOINT_IN, LIBUSB_ENDPOINT_DIR_MASK}, Error};

use super::register::*;

pub(crate) type RegisterAddress = u16;
pub(crate) type RegisterValue = u8;

#[repr(u8)]
enum Request {
    RegisterRead = 0,
    RegisterWrite = 1,
    FramerIfControl = 2,
}

impl From<Request> for u8 {
    fn from(request: Request) -> Self {
        request as u8
    }
}

pub struct Access<'a, T> {
    device: &'a Device,
    address: usize,
    t: PhantomData<T>,
}

impl<'a, T> Access<'a, T> {
    fn new(device: &'a Device, address: usize) -> Self {
        Self {
            device,
            address,
            t: PhantomData::default(),
        }
    }
}

impl<T: From<u8>> Access<'_, T> {
    fn get_typed(&self, address: usize) -> Result<T> {
        assert!(address < 0x10000);
        Ok(T::from(self.device.register_read(address as RegisterAddress)?))
    }

    pub fn read(&self) -> Result<T> {
        self.get_typed(self.address)
    }
}

// TODO: Access require Default for each register?
// Best not to do it per type, but per individual
// register. So maybe best to have the default()
// implementations reach into an array of default
// values while we still have the address in-hand.

impl<T: From<u8> + Into<u8>> Access<'_, T> {
    fn set_typed(&self, address: usize, value: T) -> Result<()> {
        assert!(address < 0x10000);
        self.device.register_write(address as RegisterAddress, value.into())
    }

    pub fn write<F>(&self, f: F) -> Result<()>
    where
        F: FnOnce(T) -> T,
    {
        let value = T::from(0);
        let new_value = f(value);
        self.set_typed(self.address, new_value)
    }

    pub fn modify<F>(&self, f: F) -> Result<()>
    where
        F: FnOnce(T) -> T,
    {
        let value = self.read()?;
        let new_value = f(value);
        self.set_typed(self.address, new_value)
    }
}

///////////////////////////////////////////////////////////////////////
// Timeslot

pub struct Timeslot<'a> {
    device: &'a Device,
    channel: usize,
    index: usize,
}

impl<'a> Timeslot<'a> {
    fn new(device: &'a Device, channel: usize, index: usize) -> Self {
        Self {
            device,
            channel,
            index,
        }
    }

    fn access<T>(&self, block_offset: usize) -> Access<'_, T> {
        Access::new(self.device, Addressing::channel_nxxx_timeslot(self.channel, block_offset, self.index))
    }

    fn access_rds0mr<T>(&self) -> Access<'_, T> {
        Access::new(self.device, Addressing::rds0mr(self.channel, self.index))
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn rds0mr(&self) -> Access<RDS0MR> { self.access_rds0mr() }
    pub fn tds0mr(&self) -> Access<TDS0MR> { self.access(0x1d0) }
    pub fn tccr  (&self) -> Access<TCCR>   { self.access(0x300) }
    pub fn tucr  (&self) -> Access<TUCR>   { self.access(0x320) }
    pub fn tscr  (&self) -> Access<TSCR>   { self.access(0x340) }
    pub fn rccr  (&self) -> Access<RCCR>   { self.access(0x360) }
    pub fn rucr  (&self) -> Access<RUCR>   { self.access(0x380) }
    pub fn rscr  (&self) -> Access<RSCtR>  { self.access(0x3a0) }
    pub fn rssr  (&self) -> Access<RSSR>   { self.access(0x3c0) }
    pub fn rsar  (&self) -> Access<RSAR>   { self.access(0x500) }
}

pub struct Timeslots<'a> {
    device: &'a Device,
    channel: usize,
    n: usize,
}

impl<'a> Timeslots<'a> {
    const COUNT: usize = 24;

    fn new(device: &'a Device, channel: usize) -> Self {
        Self {
            device,
            channel,
            n: 0,
        }
    }
}

impl<'a> Iterator for Timeslots<'a> {
    type Item = Timeslot<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.n < Timeslots::COUNT {
            let result = Timeslot::new(self.device, self.channel, self.n);
            self.n += 1;
            Some(result)
        } else {
            None
        }
    }
}

///////////////////////////////////////////////////////////////////////
// Channel

pub struct RSCRBitmap(u32);

impl RSCRBitmap {
    pub fn len(&self) -> usize {
        24
    }

    pub fn changed(&self, index: usize) -> bool {
        assert!(index < self.len());
        let bit = (self.0 >> index) & 1;
        bit != 0
    }
}

impl fmt::Debug for RSCRBitmap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = self.0.reverse_bits() >> 8;
        write!(f, "RSCR:[{:024b}]", value)
    }
}

pub struct Channel<'a> {
    device: &'a Device,
    index: usize,
}

impl<'a> Channel<'a> {
    fn new(device: &'a Device, index: usize) -> Self {
        assert!(index < Channels::COUNT);

        Self {
            device,
            index,
        }
    }

    fn access_framer<T>(&self, offset: usize) -> Access<'_, T> {
        Access::new(self.device, Addressing::channel_nxxx(self.index, offset))
    }

    fn access_liu<T>(&self, offset: usize) -> Access<'_, T> {
        Access::new(self.device, Addressing::channel_0fnx(self.index, offset))
    }

    pub fn device(&self) -> &'a Device {
        self.device
    }
    
    pub fn index(&self) -> usize {
        self.index
    }

    pub fn timeslot(&self, index: usize) -> Timeslot {
        Timeslot::new(self.device, self.index, index)
    }

    pub fn timeslots(&self) -> Timeslots {
        Timeslots::new(self.device, self.index)
    }

    pub fn csr     (&self) -> Access<CSR>      { self.access_framer(0x100) }
    pub fn licr    (&self) -> Access<LICR>     { self.access_framer(0x101) }
    pub fn fsr     (&self) -> Access<FSR>      { self.access_framer(0x107) }
    pub fn agr     (&self) -> Access<AGR>      { self.access_framer(0x108) }
    pub fn smr     (&self) -> Access<SMR>      { self.access_framer(0x109) }
    pub fn tsdlsr  (&self) -> Access<TSDLSR>   { self.access_framer(0x10a) }
    pub fn fcr     (&self) -> Access<FCR>      { self.access_framer(0x10b) }
    pub fn rsdlsr  (&self) -> Access<RSDLSR>   { self.access_framer(0x10c) }
    pub fn rscr0   (&self) -> Access<RSChR>    { self.rscr(0) }
    pub fn rscr1   (&self) -> Access<RSChR>    { self.rscr(1) }
    pub fn rscr2   (&self) -> Access<RSChR>    { self.rscr(2) }
    pub fn rifr    (&self) -> Access<RIFR>     { self.access_framer(0x112) }
    pub fn dlcr1   (&self) -> Access<DLCR>     { self.access_framer(0x113) }
    pub fn tdlbcr1 (&self) -> Access<TDLBCR>   { self.access_framer(0x114) }
    pub fn rdlbcr1 (&self) -> Access<RDLBCR>   { self.rdlbcr(0) }
    pub fn sbcr    (&self) -> Access<SBCR>     { self.access_framer(0x116) }
    pub fn fifolr  (&self) -> Access<FIFOLR>   { self.access_framer(0x117) }
    pub fn icr     (&self) -> Access<ICR>      { self.access_framer(0x11a) }
    pub fn lapdsr  (&self) -> Access<LAPDSR>   { self.access_framer(0x11b) }
    pub fn ciagr   (&self) -> Access<CIAGR>    { self.access_framer(0x11c) }
    pub fn prcr    (&self) -> Access<PRCR>     { self.access_framer(0x11d) }
    pub fn gccr    (&self) -> Access<GCCR>     { self.access_framer(0x11e) }
    pub fn ticr    (&self) -> Access<TICR>     { self.access_framer(0x120) }
    pub fn bertcsr0(&self) -> Access<BERTCSR0> { self.access_framer(0x121) }
    pub fn ricr    (&self) -> Access<RICR>     { self.access_framer(0x122) }
    pub fn bertcsr1(&self) -> Access<BERTCSR1> { self.access_framer(0x123) }
    pub fn lccr0   (&self) -> Access<LCCR0>    { self.access_framer(0x124) }
    pub fn tlcr    (&self) -> Access<TLCR>     { self.access_framer(0x125) }
    pub fn rlacr0  (&self) -> Access<RLACR>    { self.access_framer(0x126) }
    pub fn rldcr0  (&self) -> Access<RLDCR>    { self.access_framer(0x127) }
    pub fn rlcds   (&self) -> Access<RLCDS>    { self.access_framer(0x128) }
    pub fn dder    (&self) -> Access<DDER>     { self.access_framer(0x129) }
    pub fn lccr1   (&self) -> Access<LCCR>     { self.access_framer(0x12a) }
    pub fn rlacr1  (&self) -> Access<RLACR>    { self.access_framer(0x12b) }
    pub fn rldcr1  (&self) -> Access<RLDCR>    { self.access_framer(0x12c) }
    pub fn lccr2   (&self) -> Access<LCCR>     { self.access_framer(0x12d) }
    pub fn rlacr2  (&self) -> Access<RLACR>    { self.access_framer(0x12e) }
    pub fn rldcr2  (&self) -> Access<RLDCR>    { self.access_framer(0x12f) }
    pub fn tlcgs   (&self) -> Access<TLCGS>    { self.access_framer(0x140) }
    pub fn lcts    (&self) -> Access<LCTS>     { self.access_framer(0x141) }
    pub fn tsprmcr (&self) -> Access<TSPRMCR>  { self.access_framer(0x142) }
    pub fn dlcr2   (&self) -> Access<DLCR>     { self.access_framer(0x143) }
    pub fn tdlbcr2 (&self) -> Access<TDLBCR>   { self.access_framer(0x144) }
    pub fn rdlbcr2 (&self) -> Access<RDLBCR>   { self.rdlbcr(1) }
    pub fn lccr3   (&self) -> Access<LCCR>     { self.access_framer(0x146) }
    pub fn rlacr3  (&self) -> Access<RLACR>    { self.access_framer(0x147) }
    pub fn rldcr3  (&self) -> Access<RLDCR>    { self.access_framer(0x148) }
    pub fn lccr4   (&self) -> Access<LCCR>     { self.access_framer(0x149) }
    pub fn rlacr4  (&self) -> Access<RLACR>    { self.access_framer(0x14a) }
    pub fn rldcr4  (&self) -> Access<RLDCR>    { self.access_framer(0x14b) }
    pub fn lccr5   (&self) -> Access<LCCR>     { self.access_framer(0x14c) }
    pub fn rlacr5  (&self) -> Access<RLACR>    { self.access_framer(0x14d) }
    pub fn rldcr5  (&self) -> Access<RLDCR>    { self.access_framer(0x14e) }
    pub fn lccr6   (&self) -> Access<LCCR>     { self.access_framer(0x14f) }
    pub fn rlacr6  (&self) -> Access<RLACR>    { self.access_framer(0x150) }
    pub fn rldcr6  (&self) -> Access<RLDCR>    { self.access_framer(0x151) }
    pub fn dlcr3   (&self) -> Access<DLCR>     { self.access_framer(0x153) }
    pub fn tdlbcr3 (&self) -> Access<TDLBCR>   { self.access_framer(0x154) }
    pub fn rdlbcr3 (&self) -> Access<RDLBCR>   { self.rdlbcr(2) }
    pub fn lccr7   (&self) -> Access<LCCR>     { self.access_framer(0x156) }
    pub fn rlacr7  (&self) -> Access<RLACR>    { self.access_framer(0x157) }
    pub fn rldcr7  (&self) -> Access<RLDCR>    { self.access_framer(0x158) }
    pub fn bcr     (&self) -> Access<BCR>      { self.access_framer(0x163) }
    pub fn boccr   (&self) -> Access<BOCCR>    { self.access_framer(0x170) }
    pub fn rfdlr   (&self) -> Access<RFDLR>    { self.access_framer(0x171) }
    pub fn rfdlmr1 (&self) -> Access<RFDLMR>   { self.access_framer(0x172) }
    pub fn rfdlmr2 (&self) -> Access<RFDLMR>   { self.access_framer(0x173) }
    pub fn rfdlmr3 (&self) -> Access<RFDLMR>   { self.access_framer(0x174) }
    pub fn tfdlr   (&self) -> Access<TFDLR>    { self.access_framer(0x175) }
    pub fn tbcr    (&self) -> Access<TBCR>     { self.access_framer(0x176) }

    pub fn rscr(&self, index: usize) -> Access<RSChR> {
        const MAP: [usize; 3] = [0x10d, 0x10e, 0x10f];
        assert!(index < MAP.len());
        self.access_framer(MAP[index])
    }

    pub fn rscr_bitmap(&self) -> Result<RSCRBitmap> {
        let mut result = 0u32;
        for i in 0..3 {
            let rscr = self.rscr(i).read()?.Ch().reverse_bits();
            result |= (rscr as u32) << (i * 8);
        }
        Ok(RSCRBitmap(result))
    }

    pub fn rdlbcr(&self, index: usize) -> Access<RDLBCR> {
        const MAP: [usize; 3] = [0x115, 0x145, 0x155];
        assert!(index < MAP.len());
        self.access_framer(MAP[index])
    }

    // Receive Signaling Array Registers

    pub fn rsar(&self, index: usize) -> Access<RSAR> {
        assert!(index < 24);
        self.access_framer(0x500 + index)
    }

    // LAPD buffers 0, 1

    pub fn lapdbcr0(&self, index: usize) -> Access<LAPDBCR> {
        assert!(index < 96);
        self.access_framer(0x600 + index)
    }

    pub fn lapdbcr1(&self, index: usize) -> Access<LAPDBCR> {
        assert!(index < 96);
        self.access_framer(0x700 + index)
    }

    // Performance Monitors (PMON)

    pub fn rlcvcu  (&self) -> Access<RLCVCU>   { self.access_framer(0x900) }
    pub fn rlcvcl  (&self) -> Access<RLCVCL>   { self.access_framer(0x901) }
    pub fn rfaecu  (&self) -> Access<RFAECU>   { self.access_framer(0x902) }
    pub fn rfaecl  (&self) -> Access<RFAECL>   { self.access_framer(0x903) }
    pub fn rsefc   (&self) -> Access<RSEFC>    { self.access_framer(0x904) }
    pub fn rsbbecu (&self) -> Access<RSBBECU>  { self.access_framer(0x905) }
    pub fn rsbbecl (&self) -> Access<RSBBECL>  { self.access_framer(0x906) }
    pub fn rsc     (&self) -> Access<RSC>      { self.access_framer(0x909) }
    pub fn rlfc    (&self) -> Access<RLFC>     { self.access_framer(0x90a) }
    pub fn rcfac   (&self) -> Access<RCFAC>    { self.access_framer(0x90b) }
    pub fn lfcsec1 (&self) -> Access<LFCSEC1>  { self.access_framer(0x90c) }
    pub fn pbecu   (&self) -> Access<PBECU>    { self.access_framer(0x90d) }
    pub fn pbecl   (&self) -> Access<PBECL>    { self.access_framer(0x90e) }
    pub fn tsc     (&self) -> Access<TSC>      { self.access_framer(0x90f) }
    pub fn ezvcu   (&self) -> Access<EZVCU>    { self.access_framer(0x910) }
    pub fn ezvcl   (&self) -> Access<EZVCL>    { self.access_framer(0x911) }
    pub fn lfcsec2 (&self) -> Access<LFCSEC2>  { self.access_framer(0x91c) }
    pub fn lfcsec3 (&self) -> Access<LFCSEC3>  { self.access_framer(0x92c) }

    // Interrupts and Status

    pub fn bisr    (&self) -> Access<BISR>     { self.access_framer(0xb00) }
    pub fn bier    (&self) -> Access<BIER>     { self.access_framer(0xb01) }
    pub fn aeisr   (&self) -> Access<AEISR>    { self.access_framer(0xb02) }
    pub fn aeier   (&self) -> Access<AEIER>    { self.access_framer(0xb03) }
    pub fn fisr    (&self) -> Access<FISR>     { self.access_framer(0xb04) }
    pub fn fier    (&self) -> Access<FIER>     { self.access_framer(0xb05) }
    pub fn dlsr1   (&self) -> Access<DLSRx>    { self.dlsr(0) }
    pub fn dlier1  (&self) -> Access<DLIERx>   { self.access_framer(0xb07) }
    pub fn sbisr   (&self) -> Access<SBISR>    { self.access_framer(0xb08) }
    pub fn sbier   (&self) -> Access<SBIER>    { self.access_framer(0xb09) }
    pub fn rlcisr0 (&self) -> Access<RLCISRx>  { self.access_framer(0xb0a) }
    pub fn rlcier0 (&self) -> Access<RLCIERx>  { self.access_framer(0xb0b) }
    pub fn exzsr   (&self) -> Access<EXZSR>    { self.access_framer(0xb0e) }
    pub fn exzer   (&self) -> Access<EXZER>    { self.access_framer(0xb0f) }
    pub fn ss7sr1  (&self) -> Access<SS7SRx>   { self.ss7sr(0) }
    pub fn ss7er1  (&self) -> Access<SS7ERx>   { self.access_framer(0xb11) }
    pub fn rlcisr  (&self) -> Access<RLCISR>   { self.access_framer(0xb12) }
    pub fn rlcier  (&self) -> Access<RLCIER>   { self.access_framer(0xb13) }
    pub fn rlcisr1 (&self) -> Access<RLCISRx>  { self.access_framer(0xb14) }
    pub fn rlcier1 (&self) -> Access<RLCIERx>  { self.access_framer(0xb15) }
    pub fn dlsr2   (&self) -> Access<DLSRx>    { self.dlsr(1) }
    pub fn dlier2  (&self) -> Access<DLIERx>   { self.access_framer(0xb17) }
    pub fn ss7sr2  (&self) -> Access<SS7SRx>   { self.ss7sr(1) }
    pub fn ss7er2  (&self) -> Access<SS7ERx>   { self.access_framer(0xb19) }
    pub fn rlcisr2 (&self) -> Access<RLCISRx>  { self.access_framer(0xb1a) }
    pub fn rlcier2 (&self) -> Access<RLCIERx>  { self.access_framer(0xb1b) }
    pub fn rlcisr3 (&self) -> Access<RLCISRx>  { self.access_framer(0xb1c) }
    pub fn rlcier3 (&self) -> Access<RLCIERx>  { self.access_framer(0xb1d) }
    pub fn rlcisr4 (&self) -> Access<RLCISRx>  { self.access_framer(0xb1e) }
    pub fn rlcier4 (&self) -> Access<RLCIERx>  { self.access_framer(0xb1f) }
    pub fn rlcisr5 (&self) -> Access<RLCISRx>  { self.access_framer(0xb20) }
    pub fn rlcier5 (&self) -> Access<RLCIERx>  { self.access_framer(0xb21) }
    pub fn rlcisr6 (&self) -> Access<RLCISRx>  { self.access_framer(0xb22) }
    pub fn rlcier6 (&self) -> Access<RLCIERx>  { self.access_framer(0xb23) }
    pub fn rlcisr7 (&self) -> Access<RLCISRx>  { self.access_framer(0xb24) }
    pub fn rlcier7 (&self) -> Access<RLCIERx>  { self.access_framer(0xb25) }
    pub fn dlsr3   (&self) -> Access<DLSRx>    { self.dlsr(2) }
    pub fn dlier3  (&self) -> Access<DLIERx>   { self.access_framer(0xb27) }
    pub fn ss7sr3  (&self) -> Access<SS7SRx>   { self.ss7sr(2) }
    pub fn ss7er3  (&self) -> Access<SS7ERx>   { self.access_framer(0xb29) }
    pub fn ciasr   (&self) -> Access<CIASR>    { self.access_framer(0xb40) }
    pub fn ciaier  (&self) -> Access<CIAIER>   { self.access_framer(0xb41) }
    pub fn bocisr  (&self) -> Access<BOCISR>   { self.access_framer(0xb70) }
    pub fn bocier  (&self) -> Access<BOCIER>   { self.access_framer(0xb71) }
    pub fn bocuisr (&self) -> Access<BOCUISR>  { self.access_framer(0xb74) }
    pub fn bocuier (&self) -> Access<BOCUIER>  { self.access_framer(0xb75) }

    pub fn dlsr(&self, index: usize) -> Access<DLSRx> {
        const MAP: [usize; 3] = [0xb06, 0xb16, 0xb26];
        assert!(index < MAP.len());
        self.access_framer(MAP[index])
    }

    pub fn ss7sr(&self, index: usize) -> Access<SS7SRx> {
        const MAP: [usize; 3] = [0xb10, 0xb18, 0xb28];
        assert!(index < MAP.len());
        self.access_framer(MAP[index])
    }

    // LIU

    pub fn liuccr0 (&self) -> Access<LIUCCR0>  { self.access_liu(0x0) }
    pub fn liuccr1 (&self) -> Access<LIUCCR1>  { self.access_liu(0x1) }
    pub fn liuccr2 (&self) -> Access<LIUCCR2>  { self.access_liu(0x2) }
    pub fn liuccr3 (&self) -> Access<LIUCCR3>  { self.access_liu(0x3) }
    pub fn liuccier(&self) -> Access<LIUCCIER> { self.access_liu(0x4) }
    pub fn liuccsr (&self) -> Access<LIUCCSR>  { self.access_liu(0x5) }
    pub fn liuccisr(&self) -> Access<LIUCCISR> { self.access_liu(0x6) }
    pub fn liuccccr(&self) -> Access<LIUCCCCR> { self.access_liu(0x7) }
    pub fn liuccar1(&self) -> Access<LIUCCAR>  { self.access_liu(0x8) }
    pub fn liuccar2(&self) -> Access<LIUCCAR>  { self.access_liu(0x9) }
    pub fn liuccar3(&self) -> Access<LIUCCAR>  { self.access_liu(0xa) }
    pub fn liuccar4(&self) -> Access<LIUCCAR>  { self.access_liu(0xb) }
    pub fn liuccar5(&self) -> Access<LIUCCAR>  { self.access_liu(0xc) }
    pub fn liuccar6(&self) -> Access<LIUCCAR>  { self.access_liu(0xd) }
    pub fn liuccar7(&self) -> Access<LIUCCAR>  { self.access_liu(0xe) }
    pub fn liuccar8(&self) -> Access<LIUCCAR>  { self.access_liu(0xf) }
}

pub struct Channels<'a> {
    device: &'a Device,
    n: usize,
}

impl<'a> Channels<'a> {
    const COUNT: usize = 8;

    fn new(device: &'a Device) -> Self {
        Self {
            device,
            n: 0,
        }
    }
}

impl<'a> Iterator for Channels<'a> {
    type Item = Channel<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.n < Self::COUNT {
            let result = Channel::new(self.device, self.n);
            self.n += 1;
            Some(result)
        } else {
            None
        }
    }
}

///////////////////////////////////////////////////////////////////////
// Experimental USB Async Thing

use rusb::ffi::libusb_transfer;

// TODO: Borrowed from rusb::ffi, because it's pub(crate).
#[doc(hidden)]
pub(crate) fn from_libusb(err: i32) -> rusb::Error {
    use rusb::ffi::constants::*;

    match err {
        LIBUSB_ERROR_IO => Error::Io,
        LIBUSB_ERROR_INVALID_PARAM => Error::InvalidParam,
        LIBUSB_ERROR_ACCESS => Error::Access,
        LIBUSB_ERROR_NO_DEVICE => Error::NoDevice,
        LIBUSB_ERROR_NOT_FOUND => Error::NotFound,
        LIBUSB_ERROR_BUSY => Error::Busy,
        LIBUSB_ERROR_TIMEOUT => Error::Timeout,
        LIBUSB_ERROR_OVERFLOW => Error::Overflow,
        LIBUSB_ERROR_PIPE => Error::Pipe,
        LIBUSB_ERROR_INTERRUPTED => Error::Interrupted,
        LIBUSB_ERROR_NO_MEM => Error::NoMem,
        LIBUSB_ERROR_NOT_SUPPORTED => Error::NotSupported,
        LIBUSB_ERROR_OTHER | _ => Error::Other,
    }
}

// struct CallbackData<T: UsbContext> {
struct CallbackData {
    // context: T,
    sender: Sender<AsyncThingMessage>,
}

pub extern "system" fn async_thing_callback(
    transfer: *mut libusb_transfer,
) {
    use rusb::ffi::constants::*;

    let status = unsafe { (*transfer).status };

    let result = unsafe {
        use rusb::ffi::*;

        libusb_submit_transfer(transfer)
    };

    match status {
        LIBUSB_TRANSFER_COMPLETED => {},
        LIBUSB_TRANSFER_TIMED_OUT => {},
        _ => {
            let s = match status {
                LIBUSB_TRANSFER_COMPLETED => "completed",
                LIBUSB_TRANSFER_ERROR => "error",
                LIBUSB_TRANSFER_TIMED_OUT => "timed out",
                LIBUSB_TRANSFER_CANCELLED => "cancelled",
                LIBUSB_TRANSFER_STALL => "stall",
                LIBUSB_TRANSFER_NO_DEVICE => "no device",
                LIBUSB_TRANSFER_OVERFLOW => "overflow",
                n => "???",
            };
        
            println!("callback: {s}");
        }
    }

    if result != 0 {
        eprintln!("error: libusb_submit_transfer: {:?}", result);
    }

    if status == LIBUSB_TRANSFER_COMPLETED {
        let data = unsafe { &mut *((*transfer).user_data as *mut CallbackData) };
        if let Err(e) = data.sender.send(AsyncThingMessage::Interrupt) {
            eprint!("error: data.sender.send: {:?}", e);
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum AsyncThingMessage {
    Interrupt,
}

pub struct AsyncThing {
    endpoint_address: u8,
}

impl AsyncThing {
    // pub fn illegal_bagel(&self, timeout: Duration) -> Result<usize> {
    //     let endpoint = self.interrupt_endpoint_address;
    //     if endpoint & LIBUSB_ENDPOINT_DIR_MASK != LIBUSB_ENDPOINT_IN {
    //         return Err(Error::InvalidParam);
    //     }
    //     let dev_handle = &self.handle;
    //     let mut buf = [0; 4];
    //     let mut transferred = mem::MaybeUninit::<c_int>::uninit();
    //     unsafe {
    //         match libusb_interrupt_transfer(
    //             dev_handle.as_raw(),
    //             endpoint,
    //             buf.as_mut_ptr() as *mut c_uchar,
    //             buf.len() as c_int,
    //             transferred.as_mut_ptr(),
    //             timeout.as_millis() as c_uint,
    //         ) {
    //             0 => Ok(transferred.assume_init() as usize),
    //             err if err == LIBUSB_ERROR_INTERRUPTED => {
    //                 let transferred = transferred.assume_init();
    //                 if transferred > 0 {
    //                     Ok(transferred as usize)
    //                 } else {
    //                     Err(from_libusb(err))
    //                 }
    //             }
    //             err => Err(from_libusb(err)),
    //         }
    //     }
    // }

    pub fn run(context: &mut rusb::Context, sender: Sender<AsyncThingMessage>) -> Result<()> {        
    // pub fn run(handle: &rusb::DeviceHandle<rusb::Context>, sender: Sender<AsyncThingMessage>) -> Result<()> {        
        let mut handle = open_device(context)?;

        let endpoint = LIBUSB_ENDPOINT_IN | 9;
        if endpoint & LIBUSB_ENDPOINT_DIR_MASK != LIBUSB_ENDPOINT_IN {
            return Err(Error::InvalidParam);
        }

        handle.claim_interface(0)?;

        let mut callback_data = Box::new(CallbackData {
            // context: context.borrow().clone(),
            sender,
        });

        let dev_handle = &handle;
        let mut buffer = Box::<[u8; 4]>::new([0; 4]);

        // NOTE: EVIL HACK, I'M SHARING BUFFERS BETWEEN TRANSFERS.

        unsafe {
            use rusb::ffi::*;

            let user_data = &mut *callback_data as *mut _ as *mut _;
            println!("user_data={:?}", user_data);

            for i in 0..4 {
                let transfer = libusb_alloc_transfer(0);
                libusb_fill_interrupt_transfer(
                    transfer,
                    dev_handle.as_raw(),
                    endpoint,
                    buffer.as_mut_ptr(),
                    buffer.len() as c_int,
                    async_thing_callback,
                    user_data,
                    2500
                );
                let result = libusb_submit_transfer(transfer);
                if result != 0 {
                    eprintln!("error: libusb_submit_transfer: {:?}", result);
                }
            }
        }

        let context = handle.context();

        println!("waiting for events");
        loop {
            if let Err(e) = context.handle_events(None) {
                eprintln!("error: context.handle_Events: {:?}", e);
                break;
            }
        }

        // TODO: Clean up!
        // TODO: Return error!

        Ok(())
    }
}

///////////////////////////////////////////////////////////////////////
// Device

const VENDOR_ID: u16 = 0x16d0;
const PRODUCT_ID: u16 = 0x0f3b;

pub(crate) fn open_device(context: &mut rusb::Context) -> Result<rusb::DeviceHandle<rusb::Context>> {
    // TODO: This is a surprisingly heavy way to open a specific device by VID/PID.
    for device in context.devices()?.iter() {
        if let Ok(device_descriptor) = device.device_descriptor() {
            if device_descriptor.vendor_id() == VENDOR_ID && device_descriptor.product_id() == PRODUCT_ID {
                return device.open();
            }
        }
    }

    Err(rusb::Error::NoDevice)
}

/// XRT86VX38 device interface
/// 
/// Abstracts the XRT86VX38 "uP" interface
/// 
pub struct Device {
    handle: rusb::DeviceHandle<rusb::Context>,
    timeout: Duration,
    interrupt_endpoint_address: u8,
}

pub type Result<T> = rusb::Result<T>;

impl Device {
    pub fn open(context: &mut rusb::Context) -> Result<Self> {
        let handle = open_device(context)?;
        
        Ok(Self {
                handle,
                // spans,
                timeout: Duration::from_secs(1),
                interrupt_endpoint_address: LIBUSB_ENDPOINT_IN | 9,
        })
    }

    pub(crate) fn handle(&mut self) -> &mut rusb::DeviceHandle<rusb::Context> {
        &mut self.handle
    }

    // pub fn interrupt_read(&self, timeout: Duration) -> Result<()> {
    //     let mut buf = [0; 4];
    //     self.handle.read_interrupt(self.interrupt_endpoint_address, &mut buf, timeout)?;
    //     println!("{buf:?}");
    //     Ok(())
    // }

    pub fn register_read(&self, address: RegisterAddress) -> Result<RegisterValue> {
        let request_type = rusb::request_type(rusb::Direction::In, rusb::RequestType::Vendor, rusb::Recipient::Device);

        let mut buf = [0u8; 1];
        self.handle.read_control(request_type, Request::RegisterRead.into(), 0, address, &mut buf, self.timeout)?;

        Ok(buf[0])
    }

    pub fn register_write(&self, address: RegisterAddress, value: RegisterValue) -> Result<()> {
        let request_type = rusb::request_type(rusb::Direction::Out, rusb::RequestType::Vendor, rusb::Recipient::Device);

        let buf = [0u8; 0];
        self.handle.write_control(request_type, Request::RegisterWrite.into(), value.into(), address, &buf, self.timeout)?;

        Ok(())
    }

    pub fn framer_interface_control(&self, enable: bool) -> Result<()> {
        let request_type = rusb::request_type(rusb::Direction::Out, rusb::RequestType::Vendor, rusb::Recipient::Device);

        let buf = [0u8; 0];
        self.handle.write_control(request_type, Request::FramerIfControl.into(), enable.into(), 0, &buf, self.timeout)?;

        Ok(())
    }

    fn access_liu_global<T>(&self, offset: usize) -> Access<'_, T> {
        Access::new(self, Addressing::global_0fex(offset))
    }

    fn access_global<T>(&self, offset: usize) -> Access<'_, T> {
        Access::new(self, Addressing::global(offset))
    }

    pub fn channels(&self) -> Channels {
        Channels::new(self)
    }

    // Per-Channel

    pub fn channel(&self, index: usize) -> Channel {
        Channel::new(self, index)
    }

    // LIU Global Control

    pub fn liugcr0(&self) -> Access<LIUGCR0> { self.access_liu_global(0x0) }
    pub fn liugcr1(&self) -> Access<LIUGCR1> { self.access_liu_global(0x1) }
    pub fn liugcr2(&self) -> Access<LIUGCR2> { self.access_liu_global(0x2) }
    pub fn liugcr3(&self) -> Access<LIUGCR3> { self.access_liu_global(0x4) }
    pub fn liugcr4(&self) -> Access<LIUGCR4> { self.access_liu_global(0x9) }
    pub fn liugcr5(&self) -> Access<LIUGCR5> { self.access_liu_global(0xa) }

    // Device Identification

    pub fn devid  (&self) -> Access<DEVID>   { self.access_global(0x01fe) }
    pub fn revid  (&self) -> Access<REVID>   { self.access_global(0x01ff) }
}

/// XRT86VX38 "uP" interface memory map addressing abstraction
/// 
struct Addressing {}

impl Addressing {
    fn global(offset: usize) -> usize {
        assert!(offset < 0x10000);
        offset
    }

    /// Registers of the form 0xNxxx + offset
    fn channel_nxxx(channel: usize, offset: usize) -> usize {
        assert!(channel < Channels::COUNT);
        assert!(offset < 0x1000);
        Self::global((channel << 12) | offset)
    }

    /// Registers of the form 0xNxxx where offset points to a series of timeslot registers.
    fn channel_nxxx_timeslot(channel: usize, block_offset: usize, timeslot: usize) -> usize {
        assert!(block_offset & 0xf == 0);
        assert!(timeslot < Timeslots::COUNT);
        let offset = block_offset + timeslot;
        Self::channel_nxxx(channel, offset)
    }

    /// Registers of the form 0x0fNx
    fn channel_0fnx(channel: usize, offset: usize) -> usize {
        assert!(channel < Channels::COUNT);
        assert!(offset < 16);
        let address = 0x0f00 | (channel << 4) | offset;
        Self::global(address)
    }

    /// Registers of the form 0x0fex
    fn global_0fex(offset: usize) -> usize {
        assert!(offset < 16);
        let address = 0x0fe0 | offset;
        Self::global(address)
    }

    /// The RDS0MR register set is "special", in that it has a gap at 0xN163,
    /// and skips from 0xN16f to 0xN1c0
    fn rds0mr(channel: usize, timeslot: usize) -> usize {
        const OFFSET_MAP: [usize; Timeslots::COUNT] = [
            0x15f, 0x160, 0x161, 0x162, 0x164, 0x165, 0x166, 0x167,
            0x168, 0x169, 0x16a, 0x16b, 0x16c, 0x16d, 0x16e, 0x16f,
            0x1c0, 0x1c1, 0x1c2, 0x1c3, 0x1c4, 0x1c5, 0x1c6, 0x1c7,
        ];

        assert!(timeslot < OFFSET_MAP.len());
        let offset = OFFSET_MAP[timeslot];

        Self::channel_nxxx(channel, offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addressing_channel_nxxx() {
        assert_eq!(Addressing::channel_nxxx(0, 0x000), 0x0000);
        assert_eq!(Addressing::channel_nxxx(7, 0x123), 0x7123);
        assert_eq!(Addressing::channel_nxxx(0, 0xfff), 0x0fff);
        assert_eq!(Addressing::channel_nxxx(7, 0xfff), 0x7fff);
    }

    #[test]
    #[should_panic(expected="channel < Channels::COUNT")]    
    fn addressing_channel_nxxx_bad_channel_index() {
        let _ = Addressing::channel_nxxx(8, 0x000);
    }

    #[test]
    #[should_panic(expected="offset < 0x1000")]    
    fn addressing_channel_nxxx_bad_offset() {
        let _ = Addressing::channel_nxxx(0, 0x1000);
    }

    #[test]
    fn addressing_channel_nxxx_timeslot() {
        assert_eq!(Addressing::channel_nxxx_timeslot(0, 0x300,  0), 0x0300);
        assert_eq!(Addressing::channel_nxxx_timeslot(0, 0x3c0, 23), 0x03d7);
        assert_eq!(Addressing::channel_nxxx_timeslot(7, 0x500,  8), 0x7508);
        assert_eq!(Addressing::channel_nxxx_timeslot(7, 0x1d0, 15), 0x71df);
    }

    #[test]
    #[should_panic(expected="channel < Channels::COUNT")]
    fn addressing_channel_nxxx_timeslot_bad_channel_index() {
        let _ = Addressing::channel_nxxx_timeslot(8, 0x000, 0);
    }

    #[test]
    #[should_panic(expected="timeslot < Timeslots::COUNT")]
    fn addressing_channel_nxxx_timeslot_bad_timeslot() {
        let _ = Addressing::channel_nxxx_timeslot(0, 0x000, 24);
    }

    #[test]
    #[should_panic(expected="block_offset & 0xf == 0")]
    fn addressing_channel_nxxx_timeslot_bad_offset() {
        let _ = Addressing::channel_nxxx_timeslot(0, 0x321, 0);
    }

    #[test]
    #[should_panic(expected="offset < 0x1000")]
    fn addressing_channel_nxxx_timeslot_overflow() {
        let _ = Addressing::channel_nxxx_timeslot(0, 0xff0, 16);
    }

    #[test]
    fn addressing_channel_0fnx() {
        assert_eq!(Addressing::channel_0fnx(0,  0), 0x0f00);
        assert_eq!(Addressing::channel_0fnx(0, 15), 0x0f0f);
        assert_eq!(Addressing::channel_0fnx(7,  0), 0x0f70);
        assert_eq!(Addressing::channel_0fnx(7, 15), 0x0f7f);
    }

    #[test]
    #[should_panic(expected="channel < Channels::COUNT")]
    fn addressing_channel_0fnx_bad_channel_index() {
        let _ = Addressing::channel_0fnx(8, 0);
    }

    #[test]
    #[should_panic(expected="offset < 16")]
    fn addressing_channel_0fnx_bad_offset() {
        let _ = Addressing::channel_0fnx(0, 0x10);
    }

    #[test]
    fn addressing_global_0fex() {
        assert_eq!(Addressing::global_0fex(0x0), 0x0fe0);
        assert_eq!(Addressing::global_0fex(0xf), 0x0fef);
    }

    #[test]
    #[should_panic(expected="offset < 16")]
    fn addressing_global_0fex_bad_offset() {
        let _ = Addressing::global_0fex(0x10);
    }

    #[test]
    fn addressing_global() {
        assert_eq!(Addressing::global(0x0000), 0x0000);
        assert_eq!(Addressing::global(0xffff), 0xffff);
    }

    #[test]
    #[should_panic(expected="offset < 0x10000")]
    fn addressing_global_bad_offset() {
        let _ = Addressing::global(0x10000);
    }
}
