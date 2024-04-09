// Copyright (C) 2024 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use alloc::alloc::Global;
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::alloc::Allocator;
use core::any::{type_name, TypeId};
use core::fmt::Debug;
use core::mem::size_of;
use core::ptr::{slice_from_raw_parts, slice_from_raw_parts_mut};
use acpi::{AcpiHandler, AcpiTables, AmlTable, PciConfigRegions, Sdt};
use aml::{AmlContext, AmlName, AmlValue};
use bootloader_api::BootInfo;
use futures_util::future::err;
use log::trace;
use x86_64::instructions::port::{Port, PortRead, PortWrite};
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

    let regions = PciConfigRegions::new(&tables).ok();

    let mut context = NoccioloAmlContext::new(regions);
    context.load_acpi(&tables).expect("Failed to populate ACPI information");
    context.initialize_objects().expect("Failed to initialize AML objects");
    context.debug();

    trace!("[acpi] Done.")
}

struct NoccioloAmlContext {
    context: AmlContext,
}

impl NoccioloAmlContext {
    pub fn new(regions: Option<PciConfigRegions<'static, Global>>) -> Self {
        let handler = NoccioloAmlHandler {
            regions,
        };

        Self {
            context: AmlContext::new(Box::new(handler), aml::DebugVerbosity::None),
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
        let mut data = Vec::new();
        self.context.namespace.traverse(|name, level| {
            trace!("[traverse key] {name} {:?}", level.typ);

            for (seg, val) in &level.values {
                data.push((name.clone(), seg.clone(), val.clone()));
            }

            Ok(true)
        }).expect("Failed to traverse AML namespace");

        for (name, seg, val) in data {
            let value = self.context.namespace.get(val);

            // match value {
            //     Ok(AmlValue::Buffer(..)) | Ok(AmlValue::Method {..})
            //         | Ok(AmlValue::Package(..)) => {
            //         trace!("[traverse val] {name} {seg:?} <truncated>");
            //
            //     }
            //
            //     Ok(val) => trace!("[traverse val] {name} {seg:?} {val:?}"),
            //
            //     _ => trace!("[traverse val] {name} {seg:?} {value:?}"),
            // }

            trace!("[traverse val] {name} {seg:?} {value:?}")
        }
    }
}

struct NoccioloAmlHandler  {
    regions: Option<PciConfigRegions<'static, Global>>,
}

impl aml::Handler for NoccioloAmlHandler {
    fn read_u8(&self, address: usize) -> u8 {
        aml_read(address)
    }

    fn read_u16(&self, address: usize) -> u16 {
        aml_read(address)
    }

    fn read_u32(&self, address: usize) -> u32 {
        aml_read(address)
    }

    fn read_u64(&self, address: usize) -> u64 {
        aml_read(address)
    }

    fn write_u8(&mut self, address: usize, value: u8) {
        aml_write(address, value)
    }

    fn write_u16(&mut self, address: usize, value: u16) {
        aml_write(address, value)
    }

    fn write_u32(&mut self, address: usize, value: u32) {
        aml_write(address, value)
    }

    fn write_u64(&mut self, address: usize, value: u64) {
        aml_write(address, value)
    }

    fn read_io_u8(&self, port: u16) -> u8 {
        aml_read_port(port)
    }

    fn read_io_u16(&self, port: u16) -> u16 {
        aml_read_port(port)
    }

    fn read_io_u32(&self, port: u16) -> u32 {
        aml_read_port(port)
    }

    fn write_io_u8(&self, port: u16, value: u8) {
        aml_write_port(port, value);
    }

    fn write_io_u16(&self, port: u16, value: u16) {
        aml_write_port(port, value);
    }

    fn write_io_u32(&self, port: u16, value: u32) {
        aml_write_port(port, value);
    }

    fn read_pci_u8(&self, segment: u16, bus: u8, device: u8, function: u8, offset: u16) -> u8 {
        aml_read_pci(PciRequest { segment, bus, device, function, offset })
    }

    fn read_pci_u16(&self, segment: u16, bus: u8, device: u8, function: u8, offset: u16) -> u16 {
        aml_read_pci(PciRequest { segment, bus, device, function, offset })
    }

    fn read_pci_u32(&self, segment: u16, bus: u8, device: u8, function: u8, offset: u16) -> u32 {
        aml_read_pci(PciRequest { segment, bus, device, function, offset })
    }

    fn write_pci_u8(&self, segment: u16, bus: u8, device: u8, function: u8, offset: u16, value: u8) {
        aml_write_pci(PciRequest { segment, bus, device, function, offset }, value)
    }

    fn write_pci_u16(&self, segment: u16, bus: u8, device: u8, function: u8, offset: u16, value: u16) {
        aml_write_pci(PciRequest { segment, bus, device, function, offset }, value)
    }

    fn write_pci_u32(&self, segment: u16, bus: u8, device: u8, function: u8, offset: u16, value: u32) {
        aml_write_pci(PciRequest { segment, bus, device, function, offset }, value)
    }
}

fn aml_read<T>(address: usize) -> T
        where T: Debug + Copy {
    trace!("Reading at address 0x{address:x} type {}", type_name::<T>());

    let mapping = unsafe { NoccioloAcpiHandler.map_physical_region::<T>(address, size_of::<T>()) };

    unsafe { *mapping.virtual_start().as_ptr() }
}

fn aml_write<T>(address: usize, value: T)
    where T: Debug + Copy {
    trace!("Writing at address 0x{address:x} type {} value {value:?}", type_name::<T>());

    let mapping = unsafe { NoccioloAcpiHandler.map_physical_region::<T>(address, size_of::<T>()) };

    *unsafe { &mut *mapping.virtual_start().as_ptr() } = value;
}

fn aml_read_pci<T>(request: PciRequest) -> T
        where T: Debug + Copy + PortRead {
    trace!("Reading PCI {request:?} type {}", type_name::<T>());

    let address = request.address();

    unsafe {
        let mut port = Port::new(0xCF8);
        port.write(address);
    }

    unsafe {
        let mut port = Port::new(0xCF8);
        port.read()
    }
}

fn aml_write_pci<T>(request: PciRequest, value: T)
        where T: Debug + Copy + PortWrite {
    trace!("Writing PCI {request:?} type {} value {value:?}", type_name::<T>())
}

fn aml_read_port<T>(port: u16) -> T
        where T: Debug + Copy + PortRead {
    trace!("Reading I/O port 0x{port:x} type {}", type_name::<T>());

    let mut port = Port::new(port);
    unsafe { port.read() }
}

fn aml_write_port<T>(port: u16, value: T)
    where T: Debug + Copy + PortWrite {
    trace!("Writing I/O port 0x{port:x} type {} value {value:?}", type_name::<T>());

    let mut port = Port::new(port);
    unsafe { port.write(value) }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd)]
struct PciRequest {
    pub segment: u16,
    pub bus: u8,
    pub device: u8,
    pub function: u8,
    pub offset: u16,
}

impl PciRequest {
    pub const fn address(&self) -> u32 {
        ((self.bus as u32) << 16)
            | ((self.device as u32) << 11)
            | ((self.function as u32) << 8)
            | (self.offset as u32 & 0xFC)
            | (0x80000000u32)
    }
}
