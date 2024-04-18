// Copyright (C) 2024 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use bootloader_api::BootInfo;
use log::trace;

mod io;
mod local;

pub use io::IOApic;
use local::LocalApic;
use x86_64::instructions::interrupts::without_interrupts;

#[derive(Debug, Clone, Copy)]
pub enum ApicError {

}

pub(crate) fn init(boot_info: &BootInfo) -> Result<(), ApicError> {
    trace!("Initializing APIC");

    let mut local = LocalApic::new(boot_info);
    local.initialize();
    local.do_test_stuff();

    without_interrupts(|| {
        let mut io = IOApic::new(&local);
        io.initialize();
        io.publish();
    });

    trace!("APIC has ID {} and version {:x}", local.id(), local.version());

    Ok(())
}
