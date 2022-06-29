#![allow(non_snake_case)]

use core::{marker::PhantomData, fmt};

use crate::register::*;

pub type RegisterAddress = u16;
pub type RegisterValue = u8;

#[derive(Debug)]
pub enum Error {
}

pub type Result<T> = core::result::Result<T, Error>;

pub trait Xyz {
    fn register_read(&self, address: RegisterAddress) -> Result<RegisterValue>;
    fn register_write(&self, address: RegisterAddress, value: RegisterValue) -> Result<()>;
}

pub struct Access<'a, D, T>
where D: Xyz,
{
    device: &'a D,
    address: usize,
    t: PhantomData<T>,
}

impl<'a, D, T> Access<'a, D, T>
where D: Xyz,
{
    fn new(device: &'a D, address: usize) -> Self {
        Self {
            device,
            address,
            t: PhantomData::default(),
        }
    }
}

impl<D, T> Access<'_, D, T>
where D: Xyz,
      T: From<u8>,
{
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

impl<D, T> Access<'_, D, T>
where D: Xyz,
      T: From<u8> + Into<u8>,
{
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

pub struct Timeslot<'a, D>
where D: Xyz,
{
    device: &'a D,
    channel: usize,
    index: usize,
}

impl<'a, D> Timeslot<'a, D>
where D: Xyz,
{
    fn new(device: &'a D, channel: usize, index: usize) -> Self {
        Self {
            device,
            channel,
            index,
        }
    }

    fn access<T>(&self, block_offset: usize) -> Access<'_, D, T> {
        Access::new(self.device, Addressing::channel_nxxx_timeslot(self.channel, block_offset, self.index))
    }

    fn access_rds0mr<T>(&self) -> Access<'_, D, T> {
        Access::new(self.device, Addressing::rds0mr(self.channel, self.index))
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn rds0mr(&self) -> Access<D, RDS0MR> { self.access_rds0mr() }
    pub fn tds0mr(&self) -> Access<D, TDS0MR> { self.access(0x1d0) }
    pub fn tccr  (&self) -> Access<D, TCCR>   { self.access(0x300) }
    pub fn tucr  (&self) -> Access<D, TUCR>   { self.access(0x320) }
    pub fn tscr  (&self) -> Access<D, TSCR>   { self.access(0x340) }
    pub fn rccr  (&self) -> Access<D, RCCR>   { self.access(0x360) }
    pub fn rucr  (&self) -> Access<D, RUCR>   { self.access(0x380) }
    pub fn rscr  (&self) -> Access<D, RSCtR>  { self.access(0x3a0) }
    pub fn rssr  (&self) -> Access<D, RSSR>   { self.access(0x3c0) }
    pub fn rsar  (&self) -> Access<D, RSAR>   { self.access(0x500) }
}

const TIMESLOTS_COUNT: usize = 24;

pub struct Timeslots<'a, D>
where D: Xyz,
{
    device: &'a D,
    channel: usize,
    n: usize,
}

impl<'a, D> Timeslots<'a, D>
where D: Xyz,
{
    fn new(device: &'a D, channel: usize) -> Self {
        Self {
            device,
            channel,
            n: 0,
        }
    }
}

impl<'a, D> Iterator for Timeslots<'a, D>
where D: Xyz,
{
    type Item = Timeslot<'a, D>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.n < TIMESLOTS_COUNT {
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

pub struct Channel<'a, D>
where D: Xyz,
{
    device: &'a D,
    index: usize,
}

impl<'a, D> Channel<'a, D>
where D: Xyz,
{
    fn new(device: &'a D, index: usize) -> Self {
        assert!(index < CHANNELS_COUNT);

        Self {
            device,
            index,
        }
    }

    fn access_framer<T>(&self, offset: usize) -> Access<'_, D, T> {
        Access::new(self.device, Addressing::channel_nxxx(self.index, offset))
    }

    fn access_liu<T>(&self, offset: usize) -> Access<'_, D, T> {
        Access::new(self.device, Addressing::channel_0fnx(self.index, offset))
    }

    pub fn device(&self) -> &'a D {
        self.device
    }
    
    pub fn index(&self) -> usize {
        self.index
    }

    pub fn timeslot(&self, index: usize) -> Timeslot<D> {
        Timeslot::new(self.device, self.index, index)
    }

    pub fn timeslots(&self) -> Timeslots<D> {
        Timeslots::new(self.device, self.index)
    }

    pub fn csr     (&self) -> Access<D, CSR>      { self.access_framer(0x100) }
    pub fn licr    (&self) -> Access<D, LICR>     { self.access_framer(0x101) }
    pub fn fsr     (&self) -> Access<D, FSR>      { self.access_framer(0x107) }
    pub fn agr     (&self) -> Access<D, AGR>      { self.access_framer(0x108) }
    pub fn smr     (&self) -> Access<D, SMR>      { self.access_framer(0x109) }
    pub fn tsdlsr  (&self) -> Access<D, TSDLSR>   { self.access_framer(0x10a) }
    pub fn fcr     (&self) -> Access<D, FCR>      { self.access_framer(0x10b) }
    pub fn rsdlsr  (&self) -> Access<D, RSDLSR>   { self.access_framer(0x10c) }
    pub fn rscr0   (&self) -> Access<D, RSChR>    { self.rscr(0) }
    pub fn rscr1   (&self) -> Access<D, RSChR>    { self.rscr(1) }
    pub fn rscr2   (&self) -> Access<D, RSChR>    { self.rscr(2) }
    pub fn rifr    (&self) -> Access<D, RIFR>     { self.access_framer(0x112) }
    pub fn dlcr1   (&self) -> Access<D, DLCR>     { self.access_framer(0x113) }
    pub fn tdlbcr1 (&self) -> Access<D, TDLBCR>   { self.access_framer(0x114) }
    pub fn rdlbcr1 (&self) -> Access<D, RDLBCR>   { self.rdlbcr(0) }
    pub fn sbcr    (&self) -> Access<D, SBCR>     { self.access_framer(0x116) }
    pub fn fifolr  (&self) -> Access<D, FIFOLR>   { self.access_framer(0x117) }
    pub fn icr     (&self) -> Access<D, ICR>      { self.access_framer(0x11a) }
    pub fn lapdsr  (&self) -> Access<D, LAPDSR>   { self.access_framer(0x11b) }
    pub fn ciagr   (&self) -> Access<D, CIAGR>    { self.access_framer(0x11c) }
    pub fn prcr    (&self) -> Access<D, PRCR>     { self.access_framer(0x11d) }
    pub fn gccr    (&self) -> Access<D, GCCR>     { self.access_framer(0x11e) }
    pub fn ticr    (&self) -> Access<D, TICR>     { self.access_framer(0x120) }
    pub fn bertcsr0(&self) -> Access<D, BERTCSR0> { self.access_framer(0x121) }
    pub fn ricr    (&self) -> Access<D, RICR>     { self.access_framer(0x122) }
    pub fn bertcsr1(&self) -> Access<D, BERTCSR1> { self.access_framer(0x123) }
    pub fn lccr0   (&self) -> Access<D, LCCR0>    { self.access_framer(0x124) }
    pub fn tlcr    (&self) -> Access<D, TLCR>     { self.access_framer(0x125) }
    pub fn rlacr0  (&self) -> Access<D, RLACR>    { self.access_framer(0x126) }
    pub fn rldcr0  (&self) -> Access<D, RLDCR>    { self.access_framer(0x127) }
    pub fn rlcds   (&self) -> Access<D, RLCDS>    { self.access_framer(0x128) }
    pub fn dder    (&self) -> Access<D, DDER>     { self.access_framer(0x129) }
    pub fn lccr1   (&self) -> Access<D, LCCR>     { self.access_framer(0x12a) }
    pub fn rlacr1  (&self) -> Access<D, RLACR>    { self.access_framer(0x12b) }
    pub fn rldcr1  (&self) -> Access<D, RLDCR>    { self.access_framer(0x12c) }
    pub fn lccr2   (&self) -> Access<D, LCCR>     { self.access_framer(0x12d) }
    pub fn rlacr2  (&self) -> Access<D, RLACR>    { self.access_framer(0x12e) }
    pub fn rldcr2  (&self) -> Access<D, RLDCR>    { self.access_framer(0x12f) }
    pub fn tlcgs   (&self) -> Access<D, TLCGS>    { self.access_framer(0x140) }
    pub fn lcts    (&self) -> Access<D, LCTS>     { self.access_framer(0x141) }
    pub fn tsprmcr (&self) -> Access<D, TSPRMCR>  { self.access_framer(0x142) }
    pub fn dlcr2   (&self) -> Access<D, DLCR>     { self.access_framer(0x143) }
    pub fn tdlbcr2 (&self) -> Access<D, TDLBCR>   { self.access_framer(0x144) }
    pub fn rdlbcr2 (&self) -> Access<D, RDLBCR>   { self.rdlbcr(1) }
    pub fn lccr3   (&self) -> Access<D, LCCR>     { self.access_framer(0x146) }
    pub fn rlacr3  (&self) -> Access<D, RLACR>    { self.access_framer(0x147) }
    pub fn rldcr3  (&self) -> Access<D, RLDCR>    { self.access_framer(0x148) }
    pub fn lccr4   (&self) -> Access<D, LCCR>     { self.access_framer(0x149) }
    pub fn rlacr4  (&self) -> Access<D, RLACR>    { self.access_framer(0x14a) }
    pub fn rldcr4  (&self) -> Access<D, RLDCR>    { self.access_framer(0x14b) }
    pub fn lccr5   (&self) -> Access<D, LCCR>     { self.access_framer(0x14c) }
    pub fn rlacr5  (&self) -> Access<D, RLACR>    { self.access_framer(0x14d) }
    pub fn rldcr5  (&self) -> Access<D, RLDCR>    { self.access_framer(0x14e) }
    pub fn lccr6   (&self) -> Access<D, LCCR>     { self.access_framer(0x14f) }
    pub fn rlacr6  (&self) -> Access<D, RLACR>    { self.access_framer(0x150) }
    pub fn rldcr6  (&self) -> Access<D, RLDCR>    { self.access_framer(0x151) }
    pub fn dlcr3   (&self) -> Access<D, DLCR>     { self.access_framer(0x153) }
    pub fn tdlbcr3 (&self) -> Access<D, TDLBCR>   { self.access_framer(0x154) }
    pub fn rdlbcr3 (&self) -> Access<D, RDLBCR>   { self.rdlbcr(2) }
    pub fn lccr7   (&self) -> Access<D, LCCR>     { self.access_framer(0x156) }
    pub fn rlacr7  (&self) -> Access<D, RLACR>    { self.access_framer(0x157) }
    pub fn rldcr7  (&self) -> Access<D, RLDCR>    { self.access_framer(0x158) }
    pub fn bcr     (&self) -> Access<D, BCR>      { self.access_framer(0x163) }
    pub fn boccr   (&self) -> Access<D, BOCCR>    { self.access_framer(0x170) }
    pub fn rfdlr   (&self) -> Access<D, RFDLR>    { self.access_framer(0x171) }
    pub fn rfdlmr1 (&self) -> Access<D, RFDLMR>   { self.access_framer(0x172) }
    pub fn rfdlmr2 (&self) -> Access<D, RFDLMR>   { self.access_framer(0x173) }
    pub fn rfdlmr3 (&self) -> Access<D, RFDLMR>   { self.access_framer(0x174) }
    pub fn tfdlr   (&self) -> Access<D, TFDLR>    { self.access_framer(0x175) }
    pub fn tbcr    (&self) -> Access<D, TBCR>     { self.access_framer(0x176) }

    pub fn rscr(&self, index: usize) -> Access<D, RSChR> {
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

    pub fn rdlbcr(&self, index: usize) -> Access<D, RDLBCR> {
        const MAP: [usize; 3] = [0x115, 0x145, 0x155];
        assert!(index < MAP.len());
        self.access_framer(MAP[index])
    }

    // Receive Signaling Array Registers

    pub fn rsar(&self, index: usize) -> Access<D, RSAR> {
        assert!(index < 24);
        self.access_framer(0x500 + index)
    }

    // LAPD buffers 0, 1

    pub fn lapdbcr0(&self, index: usize) -> Access<D, LAPDBCR> {
        assert!(index < 96);
        self.access_framer(0x600 + index)
    }

    pub fn lapdbcr1(&self, index: usize) -> Access<D, LAPDBCR> {
        assert!(index < 96);
        self.access_framer(0x700 + index)
    }

    // Performance Monitors (PMON)

    pub fn rlcvcu  (&self) -> Access<D, RLCVCU>   { self.access_framer(0x900) }
    pub fn rlcvcl  (&self) -> Access<D, RLCVCL>   { self.access_framer(0x901) }
    pub fn rfaecu  (&self) -> Access<D, RFAECU>   { self.access_framer(0x902) }
    pub fn rfaecl  (&self) -> Access<D, RFAECL>   { self.access_framer(0x903) }
    pub fn rsefc   (&self) -> Access<D, RSEFC>    { self.access_framer(0x904) }
    pub fn rsbbecu (&self) -> Access<D, RSBBECU>  { self.access_framer(0x905) }
    pub fn rsbbecl (&self) -> Access<D, RSBBECL>  { self.access_framer(0x906) }
    pub fn rsc     (&self) -> Access<D, RSC>      { self.access_framer(0x909) }
    pub fn rlfc    (&self) -> Access<D, RLFC>     { self.access_framer(0x90a) }
    pub fn rcfac   (&self) -> Access<D, RCFAC>    { self.access_framer(0x90b) }
    pub fn lfcsec1 (&self) -> Access<D, LFCSEC1>  { self.access_framer(0x90c) }
    pub fn pbecu   (&self) -> Access<D, PBECU>    { self.access_framer(0x90d) }
    pub fn pbecl   (&self) -> Access<D, PBECL>    { self.access_framer(0x90e) }
    pub fn tsc     (&self) -> Access<D, TSC>      { self.access_framer(0x90f) }
    pub fn ezvcu   (&self) -> Access<D, EZVCU>    { self.access_framer(0x910) }
    pub fn ezvcl   (&self) -> Access<D, EZVCL>    { self.access_framer(0x911) }
    pub fn lfcsec2 (&self) -> Access<D, LFCSEC2>  { self.access_framer(0x91c) }
    pub fn lfcsec3 (&self) -> Access<D, LFCSEC3>  { self.access_framer(0x92c) }

    // Interrupts and Status

    pub fn bisr    (&self) -> Access<D, BISR>     { self.access_framer(0xb00) }
    pub fn bier    (&self) -> Access<D, BIER>     { self.access_framer(0xb01) }
    pub fn aeisr   (&self) -> Access<D, AEISR>    { self.access_framer(0xb02) }
    pub fn aeier   (&self) -> Access<D, AEIER>    { self.access_framer(0xb03) }
    pub fn fisr    (&self) -> Access<D, FISR>     { self.access_framer(0xb04) }
    pub fn fier    (&self) -> Access<D, FIER>     { self.access_framer(0xb05) }
    pub fn dlsr1   (&self) -> Access<D, DLSRx>    { self.dlsr(0) }
    pub fn dlier1  (&self) -> Access<D, DLIERx>   { self.access_framer(0xb07) }
    pub fn sbisr   (&self) -> Access<D, SBISR>    { self.access_framer(0xb08) }
    pub fn sbier   (&self) -> Access<D, SBIER>    { self.access_framer(0xb09) }
    pub fn rlcisr0 (&self) -> Access<D, RLCISRx>  { self.access_framer(0xb0a) }
    pub fn rlcier0 (&self) -> Access<D, RLCIERx>  { self.access_framer(0xb0b) }
    pub fn exzsr   (&self) -> Access<D, EXZSR>    { self.access_framer(0xb0e) }
    pub fn exzer   (&self) -> Access<D, EXZER>    { self.access_framer(0xb0f) }
    pub fn ss7sr1  (&self) -> Access<D, SS7SRx>   { self.ss7sr(0) }
    pub fn ss7er1  (&self) -> Access<D, SS7ERx>   { self.access_framer(0xb11) }
    pub fn rlcisr  (&self) -> Access<D, RLCISR>   { self.access_framer(0xb12) }
    pub fn rlcier  (&self) -> Access<D, RLCIER>   { self.access_framer(0xb13) }
    pub fn rlcisr1 (&self) -> Access<D, RLCISRx>  { self.access_framer(0xb14) }
    pub fn rlcier1 (&self) -> Access<D, RLCIERx>  { self.access_framer(0xb15) }
    pub fn dlsr2   (&self) -> Access<D, DLSRx>    { self.dlsr(1) }
    pub fn dlier2  (&self) -> Access<D, DLIERx>   { self.access_framer(0xb17) }
    pub fn ss7sr2  (&self) -> Access<D, SS7SRx>   { self.ss7sr(1) }
    pub fn ss7er2  (&self) -> Access<D, SS7ERx>   { self.access_framer(0xb19) }
    pub fn rlcisr2 (&self) -> Access<D, RLCISRx>  { self.access_framer(0xb1a) }
    pub fn rlcier2 (&self) -> Access<D, RLCIERx>  { self.access_framer(0xb1b) }
    pub fn rlcisr3 (&self) -> Access<D, RLCISRx>  { self.access_framer(0xb1c) }
    pub fn rlcier3 (&self) -> Access<D, RLCIERx>  { self.access_framer(0xb1d) }
    pub fn rlcisr4 (&self) -> Access<D, RLCISRx>  { self.access_framer(0xb1e) }
    pub fn rlcier4 (&self) -> Access<D, RLCIERx>  { self.access_framer(0xb1f) }
    pub fn rlcisr5 (&self) -> Access<D, RLCISRx>  { self.access_framer(0xb20) }
    pub fn rlcier5 (&self) -> Access<D, RLCIERx>  { self.access_framer(0xb21) }
    pub fn rlcisr6 (&self) -> Access<D, RLCISRx>  { self.access_framer(0xb22) }
    pub fn rlcier6 (&self) -> Access<D, RLCIERx>  { self.access_framer(0xb23) }
    pub fn rlcisr7 (&self) -> Access<D, RLCISRx>  { self.access_framer(0xb24) }
    pub fn rlcier7 (&self) -> Access<D, RLCIERx>  { self.access_framer(0xb25) }
    pub fn dlsr3   (&self) -> Access<D, DLSRx>    { self.dlsr(2) }
    pub fn dlier3  (&self) -> Access<D, DLIERx>   { self.access_framer(0xb27) }
    pub fn ss7sr3  (&self) -> Access<D, SS7SRx>   { self.ss7sr(2) }
    pub fn ss7er3  (&self) -> Access<D, SS7ERx>   { self.access_framer(0xb29) }
    pub fn ciasr   (&self) -> Access<D, CIASR>    { self.access_framer(0xb40) }
    pub fn ciaier  (&self) -> Access<D, CIAIER>   { self.access_framer(0xb41) }
    pub fn bocisr  (&self) -> Access<D, BOCISR>   { self.access_framer(0xb70) }
    pub fn bocier  (&self) -> Access<D, BOCIER>   { self.access_framer(0xb71) }
    pub fn bocuisr (&self) -> Access<D, BOCUISR>  { self.access_framer(0xb74) }
    pub fn bocuier (&self) -> Access<D, BOCUIER>  { self.access_framer(0xb75) }

    pub fn rlcisr_x(&self, index: usize) -> Access<D, RLCISRx> {
        const MAP: [usize; 8] = [0xb0a, 0xb14, 0xb1a, 0xb1c, 0xb1e, 0xb20, 0xb22, 0xb24];
        assert!(index < MAP.len());
        self.access_framer(MAP[index])
    }

    pub fn rlcier_x(&self, index: usize) -> Access<D, RLCIERx> {
        const MAP: [usize; 8] = [0xb0b, 0xb15, 0xb1b, 0xb1d, 0xb1f, 0xb21, 0xb23, 0xb25];
        assert!(index < MAP.len());
        self.access_framer(MAP[index])
    }

    pub fn dlsr(&self, index: usize) -> Access<D, DLSRx> {
        const MAP: [usize; 3] = [0xb06, 0xb16, 0xb26];
        assert!(index < MAP.len());
        self.access_framer(MAP[index])
    }

    pub fn ss7sr(&self, index: usize) -> Access<D, SS7SRx> {
        const MAP: [usize; 3] = [0xb10, 0xb18, 0xb28];
        assert!(index < MAP.len());
        self.access_framer(MAP[index])
    }

    // LIU

    pub fn liuccr0 (&self) -> Access<D, LIUCCR0>  { self.access_liu(0x0) }
    pub fn liuccr1 (&self) -> Access<D, LIUCCR1>  { self.access_liu(0x1) }
    pub fn liuccr2 (&self) -> Access<D, LIUCCR2>  { self.access_liu(0x2) }
    pub fn liuccr3 (&self) -> Access<D, LIUCCR3>  { self.access_liu(0x3) }
    pub fn liuccier(&self) -> Access<D, LIUCCIER> { self.access_liu(0x4) }
    pub fn liuccsr (&self) -> Access<D, LIUCCSR>  { self.access_liu(0x5) }
    pub fn liuccisr(&self) -> Access<D, LIUCCISR> { self.access_liu(0x6) }
    pub fn liuccccr(&self) -> Access<D, LIUCCCCR> { self.access_liu(0x7) }
    pub fn liuccar1(&self) -> Access<D, LIUCCAR>  { self.access_liu(0x8) }
    pub fn liuccar2(&self) -> Access<D, LIUCCAR>  { self.access_liu(0x9) }
    pub fn liuccar3(&self) -> Access<D, LIUCCAR>  { self.access_liu(0xa) }
    pub fn liuccar4(&self) -> Access<D, LIUCCAR>  { self.access_liu(0xb) }
    pub fn liuccar5(&self) -> Access<D, LIUCCAR>  { self.access_liu(0xc) }
    pub fn liuccar6(&self) -> Access<D, LIUCCAR>  { self.access_liu(0xd) }
    pub fn liuccar7(&self) -> Access<D, LIUCCAR>  { self.access_liu(0xe) }
    pub fn liuccar8(&self) -> Access<D, LIUCCAR>  { self.access_liu(0xf) }
}

const CHANNELS_COUNT: usize = 8;

pub struct Channels<'a, D>
where D: Xyz,
{
    device: &'a D,
    n: usize,
}

impl<'a, D> Channels<'a, D>
where D: Xyz,
{
    fn new(device: &'a D) -> Self {
        Self {
            device,
            n: 0,
        }
    }
}

impl<'a, D> Iterator for Channels<'a, D>
where D: Xyz,
{
    type Item = Channel<'a, D>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.n < CHANNELS_COUNT {
            let result = Channel::new(self.device, self.n);
            self.n += 1;
            Some(result)
        } else {
            None
        }
    }
}

///////////////////////////////////////////////////////////////////////
// Device

pub trait DeviceAccess {
    fn read(&self, address: RegisterAddress) -> Result<RegisterValue>;
    fn write(&self, address: RegisterAddress, value: RegisterValue) -> Result<()>;
}

/// XRT86VX38 device interface
/// 
/// Abstracts the XRT86VX38 "uP" interface
/// 
pub struct Device<A>
where A: DeviceAccess
{
    access: A,
}

impl<A> Device<A>
where A: DeviceAccess
{
    pub fn new(access: A) -> Self {
        Self {
            access,
        }
    }

    fn access_liu_global<T>(&self, offset: usize) -> Access<'_, Self, T> {
        Access::new(self, Addressing::global_0fex(offset))
    }

    fn access_global<T>(&self, offset: usize) -> Access<'_, Self, T> {
        Access::new(self, Addressing::global(offset))
    }

    pub fn channels(&self) -> Channels<Self> {
        Channels::new(self)
    }

    // Per-Channel

    pub fn channel(&self, index: usize) -> Channel<Self> {
        Channel::new(self, index)
    }

    // LIU Global Control

    pub fn liugcr0(&self) -> Access<Self, LIUGCR0> { self.access_liu_global(0x0) }
    pub fn liugcr1(&self) -> Access<Self, LIUGCR1> { self.access_liu_global(0x1) }
    pub fn liugcr2(&self) -> Access<Self, LIUGCR2> { self.access_liu_global(0x2) }
    pub fn liugcr3(&self) -> Access<Self, LIUGCR3> { self.access_liu_global(0x4) }
    pub fn liugcr4(&self) -> Access<Self, LIUGCR4> { self.access_liu_global(0x9) }
    pub fn liugcr5(&self) -> Access<Self, LIUGCR5> { self.access_liu_global(0xa) }

    // Device Identification

    pub fn devid  (&self) -> Access<Self, DEVID>   { self.access_global(0x01fe) }
    pub fn revid  (&self) -> Access<Self, REVID>   { self.access_global(0x01ff) }
}

impl<A> Xyz for Device<A>
where A: DeviceAccess {
    fn register_read(&self, address: RegisterAddress) -> Result<RegisterValue> {
        self.access.read(address)
    }

    fn register_write(&self, address: RegisterAddress, value: RegisterValue) -> Result<()> {
        self.access.write(address, value)
    }
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
        assert!(channel < CHANNELS_COUNT);
        assert!(offset < 0x1000);
        Self::global((channel << 12) | offset)
    }

    /// Registers of the form 0xNxxx where offset points to a series of timeslot registers.
    fn channel_nxxx_timeslot(channel: usize, block_offset: usize, timeslot: usize) -> usize {
        assert!(block_offset & 0xf == 0);
        assert!(timeslot < TIMESLOTS_COUNT);
        let offset = block_offset + timeslot;
        Self::channel_nxxx(channel, offset)
    }

    /// Registers of the form 0x0fNx
    fn channel_0fnx(channel: usize, offset: usize) -> usize {
        assert!(channel < CHANNELS_COUNT);
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
        const OFFSET_MAP: [usize; TIMESLOTS_COUNT] = [
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
