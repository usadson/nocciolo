// Copyright (C) 2024 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use acpi::{AcpiTables, Sdt};
use bootloader_api::BootInfo;
use crate::serial_println;

mod handler;
mod rsdp;

pub(self) use self::handler::NoccioloAcpiHandler;

pub(super) fn init(boot_info: &'static BootInfo) {
    serial_println!("[acpi] Looking for RSDP...");
    let Some(rsdp) = rsdp::find_rsdp(boot_info) else {return};

    let state = rsdp.validate();
    serial_println!("[acpi] RSDP(valid={state:?}): {rsdp:#?}");

    if !state.is_ok() {
        return;
    }

    let tables = match unsafe { AcpiTables::from_validated_rsdp(NoccioloAcpiHandler, rsdp) } {
        Ok(tables) => tables,
        Err(e) => {
            serial_println!("[acpi] Failed to instantiate tables {e:?}");
            return;
        }
    };

    serial_println!("Platform Info: {:#?}", tables.platform_info());

    serial_println!("[acpi] Done.")
}
