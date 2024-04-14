// Copyright (C) 2024 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use bootloader_api::BootInfo;
use log::trace;

mod io;
mod local;

use io::IOApic;
use local::LocalApic;

#[derive(Debug, Clone, Copy)]
pub enum ApicError {

}

pub(crate) fn init(boot_info: &BootInfo) -> Result<(), ApicError> {
    trace!("Initializing APIC");

    let mut io = IOApic::new();
    io.initialize();

    let mut local = LocalApic::new(boot_info);
    local.initialize();
    local.do_test_stuff();

    trace!("APIC has ID {} and version {:x}", local.id(), local.version());

    Ok(())
}
