// Copyright (C) 2024 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use core::mem;
use core::mem::size_of;
use bootloader_api::BootInfo;
use acpi::{AcpiHandler, PhysicalMapping};
use acpi::rsdp::Rsdp;
use crate::device::acpi::handler::NoccioloAcpiHandler;
use crate::serial_println;

pub(super) fn find_rsdp(boot_info: &BootInfo) -> Option<PhysicalMapping<NoccioloAcpiHandler, Rsdp>> {
    if let Some(addr) = boot_info.rsdp_addr.as_ref() {
        find_rsdp_on_uefi(*addr as _)
    } else {
        find_rsdp_on_bios()
    }
}

fn find_rsdp_on_bios() -> Option<PhysicalMapping<NoccioloAcpiHandler, Rsdp>> {
    let bios_result = unsafe { Rsdp::search_for_on_bios(NoccioloAcpiHandler) };

    match bios_result {
        Ok(rsdp) => Some(rsdp),

        Err(e) => {
            serial_println!("Failed to locate RSDP on BIOS: {e:?}");
            None
        }
    }
}

fn find_rsdp_on_uefi(addr: usize) -> Option<PhysicalMapping<NoccioloAcpiHandler, Rsdp>> {
    let size = size_of::<Rsdp>();

    Some(unsafe {
        NoccioloAcpiHandler.map_physical_region::<Rsdp>(addr, size)
    })
}

