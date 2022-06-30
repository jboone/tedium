use super::device::{Device, Channel, Result, Timeslot};

pub fn registers_dump_raw(device: &Device) -> Result<()> {
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

pub fn registers_dump_debug(device: &Device) -> Result<()> {
    registers_dump_global(device)?;

    for channel in device.channels() {
        registers_dump_channel(&channel)?;
    }

    Ok(())
}

pub fn registers_dump_global(device: &Device) -> Result<()> {
    println!("Device\tDEVID=0x{:02x?}, REVID=0x{:02x?}", device.devid().read()?.DEVID(), device.revid().read()?.REVID());

    println!("Global\t{:?}", device.liugcr0().read()?);
    println!("\t{:?}", device.liugcr1().read()?);
    println!("\t{:?}", device.liugcr2().read()?);
    println!("\t{:?}", device.liugcr3().read()?);
    println!("\t{:?}", device.liugcr4().read()?);
    println!("\t{:?}", device.liugcr5().read()?);

    Ok(())
}

pub fn registers_dump_channel(channel: &Channel) -> Result<()> {
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

pub fn registers_dump_timeslot(timeslot: &Timeslot) -> Result<()> {
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
