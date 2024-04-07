// Copyright (C) 2024 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ptr::{slice_from_raw_parts, slice_from_raw_parts_mut};
use acpi::{AcpiHandler, AcpiTables, AmlTable, Sdt};
use aml::AmlContext;
use bootloader_api::BootInfo;
use futures_util::future::err;
use log::trace;
use crate::device::DeviceError;

mod handler;
mod rsdp;

pub(self) use self::handler::NoccioloAcpiHandler;

pub(super) fn init(boot_info: &'static BootInfo) {
    trace!("[acpi] Looking for RSDP...");
    let Some(rsdp) = rsdp::find_rsdp(boot_info) else {return};

    let state = rsdp.validate();
    trace!("[acpi] RSDP(valid={state:?}): {rsdp:#?}");

    if !state.is_ok() {
        return;
    }

    let tables = match unsafe { AcpiTables::from_validated_rsdp(NoccioloAcpiHandler, rsdp) } {
        Ok(tables) => tables,
        Err(e) => {
            trace!("[acpi] Failed to instantiate tables {e:?}");
            return;
        }
    };

    trace!("[acpi] Platform Info: {:#?}", tables.platform_info());

    let mut context = NoccioloAmlContext::new();
    context.load_acpi(&tables).expect("Failed to populate ACPI information");
    context.initialize_objects().expect("Failed to initialize AML objects");
    // context.debug();

    trace!("[acpi] Done.")
}

struct NoccioloAmlContext {
    context: AmlContext,
}

impl NoccioloAmlContext {
    pub fn new() -> Self {
        Self {
            context: AmlContext::new(Box::new(NoccioloAmlHandler), aml::DebugVerbosity::None),
        }
    }

    pub fn parse_table(&mut self, table: AmlTable) -> Result<(), DeviceError> {
        trace!("[acpi] [aml] Parsing AML table @ 0x{:x} sized 0x{:x}", table.address, table.length);

        let mapping = unsafe {
            NoccioloAcpiHandler.map_physical_region::<u8>(table.address, table.length as _)
        };

        let start = mapping.virtual_start().as_ptr();
        let size = mapping.region_length();
        let data = slice_from_raw_parts_mut(start, size);

        let data = unsafe { &*data };

        self.context.parse_table(data)
            .map_err(|x| DeviceError::aml(x).with_region("parse_table"))
    }

    /// As the ACPI tables describe the system context (devices, firmware, etc.), we can use it to
    /// populate the AML (ACPI Machine Language) information.
    pub fn load_acpi(&mut self, tables: &AcpiTables<NoccioloAcpiHandler>) -> Result<(), DeviceError> {
        trace!("[acpi] [aml] Loading ACPI tables to populate AML namespace...");

        // There is always one Differentiated System Description Table (DSDT).
        let dsdt = tables.dsdt()
            .map_err(|x| DeviceError::acpi(x).with_region("get dsdt"))?;
        self.parse_table(dsdt)?;

        // If the DSDT is too small, or other firmware components also provide system description
        // information, there is more information in the Secondary System Description Table (SSDT).
        for ssdt in tables.ssdts() {
            self.parse_table(ssdt)?;
        }

        trace!("[acpi] [aml] Populated...");
        Ok(())
    }

    pub fn initialize_objects(&mut self) -> Result<(), DeviceError> {
        self.context.initialize_objects()
            .map_err(|x| DeviceError::aml(x).with_region("initialize_objects"))
    }

    pub fn debug(&mut self) {
        trace!("[acpi] [aml] Traversing table...");
        self.context.namespace.traverse(|name, level| {
            trace!("[acpi] [aml] [traverse] {name}");

            Ok(true)
        }).expect("Failed to traverse AML namespace");
    }
}

struct NoccioloAmlHandler;

impl aml::Handler for NoccioloAmlHandler {
    fn read_u8(&self, address: usize) -> u8 {
        todo!()
    }

    fn read_u16(&self, address: usize) -> u16 {
        todo!()
    }

    fn read_u32(&self, address: usize) -> u32 {
        todo!()
    }

    fn read_u64(&self, address: usize) -> u64 {
        todo!()
    }

    fn write_u8(&mut self, address: usize, value: u8) {
        todo!()
    }

    fn write_u16(&mut self, address: usize, value: u16) {
        todo!()
    }

    fn write_u32(&mut self, address: usize, value: u32) {
        todo!()
    }

    fn write_u64(&mut self, address: usize, value: u64) {
        todo!()
    }

    fn read_io_u8(&self, port: u16) -> u8 {
        todo!()
    }

    fn read_io_u16(&self, port: u16) -> u16 {
        todo!()
    }

    fn read_io_u32(&self, port: u16) -> u32 {
        todo!()
    }

    fn write_io_u8(&self, port: u16, value: u8) {
        todo!()
    }

    fn write_io_u16(&self, port: u16, value: u16) {
        todo!()
    }

    fn write_io_u32(&self, port: u16, value: u32) {
        todo!()
    }

    fn read_pci_u8(&self, segment: u16, bus: u8, device: u8, function: u8, offset: u16) -> u8 {
        todo!()
    }

    fn read_pci_u16(&self, segment: u16, bus: u8, device: u8, function: u8, offset: u16) -> u16 {
        todo!()
    }

    fn read_pci_u32(&self, segment: u16, bus: u8, device: u8, function: u8, offset: u16) -> u32 {
        todo!()
    }

    fn write_pci_u8(&self, segment: u16, bus: u8, device: u8, function: u8, offset: u16, value: u8) {
        todo!()
    }

    fn write_pci_u16(&self, segment: u16, bus: u8, device: u8, function: u8, offset: u16, value: u16) {
        todo!()
    }

    fn write_pci_u32(&self, segment: u16, bus: u8, device: u8, function: u8, offset: u16, value: u32) {
        todo!()
    }
}
