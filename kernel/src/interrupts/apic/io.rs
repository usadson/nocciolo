// Copyright (C) 2024 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use core::{mem, ptr::{read_volatile, write_volatile}};

use acpi::{madt::MadtEntry, AcpiHandler, PhysicalMapping};
use log::trace;
use x86_64::PhysAddr;

use crate::{device::acpi::{NoccioloAcpiHandler, ACPI_DATA}, interrupts::InterruptIndex};

pub struct IOApic {
    mapping: PhysicalMapping<NoccioloAcpiHandler, [u32; 256]>,
    redirection_entry_count: u8,
}

impl IOApic {
    pub fn new() -> Self {
        let addr = find_io_apic_base().expect("NO IOAPIC FOUND :(");
        Self::from_addr(addr)
    }

    #[must_use]
    pub fn from_addr(addr: PhysAddr) -> Self {
        let mapping = unsafe {
            NoccioloAcpiHandler.map_physical_region(addr.as_u64() as _, 0x400)
        };

        let mut this = Self {
            mapping,
            redirection_entry_count: 0,
        };

        let redirection_entry_count = this.read_redirection_entry_count();
        debug_assert_ne!(redirection_entry_count, 0);
        this.redirection_entry_count = redirection_entry_count;

        this
    }

    pub fn initialize(&mut self) {
        trace!("I/O APIC Version {} with {} redirection entries", self.read_version(), self.redirection_entry_count);

        self.map_all_to_spurious_vectors();
        trace!("DUMPING IO APIC");
        for entry in 0..self.redirection_entry_count {
            trace!("Entry #{entry}: {:#?}", self.read_entry(entry));
        }
    }

    fn map_all_to_spurious_vectors(&mut self) {
        for index in 0..self.redirection_entry_count {
            let mut entry = self.read_entry(index);
            entry.vector = InterruptIndex::SpuriousIoApic as u8;
            entry.mask = InterruptMask::Unmasked;
            self.write_entry(index, entry);
        }
    }

    fn read_entry(&self, index: u8) -> IOApicRedirectionEntry {
        debug_assert!(index < self.redirection_entry_count, "Redirection Entry #{index} falls outside the {} entries", self.redirection_entry_count);

        let value = self.read_u64(IOApicRegister::RedirectionEntry(index));

        unsafe {
            let destination_mode = mem::transmute(((value >> 11) & 0b1) as u8);
            let destination = DestinationField::new(
                destination_mode,
                ((value >> 56) & 0b1111_1111) as u8,
            );
            IOApicRedirectionEntry {
                vector: (value & 0xFF) as u8,
                delivery_mode: mem::transmute(((value >> 8) & 0b111) as u8),
                destination_mode,
                delivery_status: mem::transmute(((value >> 12) & 0b1) as u8),
                polarity: mem::transmute(((value >> 13) & 0b1) as u8),
                remote_irr: mem::transmute(((value >> 14) & 0b1) as u8),
                trigger_mode: mem::transmute(((value >> 15) & 0b1) as u8),
                mask: mem::transmute(((value >> 16) & 0b1) as u8),
                destination,
            }
        }
    }

    fn write_entry(&mut self, index: u8, entry: IOApicRedirectionEntry) {
        debug_assert!(index < self.redirection_entry_count, "Redirection Entry #{index} falls outside the {} entries", self.redirection_entry_count);

        let value = (entry.vector as u64)
                  | ((entry.delivery_mode as u64) << 8)
                  | ((entry.destination_mode as u64) << 11)
                  | ((entry.delivery_status as u64) << 12)
                  | ((entry.polarity as u64) << 13)
                  | ((entry.remote_irr as u64) << 14)
                  | ((entry.trigger_mode as u64) << 15)
                  | ((entry.mask as u64) << 16)
                  | ((entry.destination.as_u8() as u64) << 56);

        self.write_u64(IOApicRegister::RedirectionEntry(index), value);
    }

    fn read_version(&self) -> u8 {
        (self.read_u32(IOApicRegister::Version) & 0xFF) as u8
    }

    fn read_redirection_entry_count(&self) -> u8 {
        ((self.read_u32(IOApicRegister::Version) >> 16) & 0xFF) as u8
    }

    fn read_u64(&self, reg: IOApicRegister) -> u64 {
        debug_assert!(matches!(reg, IOApicRegister::RedirectionEntry(..)));

        let lo = self.read_u32(reg);
        let hi = self.read_u32(reg.second_part_redir());
        (lo as u64) | ((hi as u64) << 32)
    }

    fn write_u64(&mut self, reg: IOApicRegister, value: u64) {
        debug_assert!(matches!(reg, IOApicRegister::RedirectionEntry(..)));

        let lo = (value & 0xFFFF_FFFF) as u32;
        let hi = ((value >> 32) & 0xFFFF_FFFF) as u32;

        self.write_u32(reg, lo);
        self.write_u32(reg.second_part_redir(), hi);
    }

    fn read_u32(&self, reg: IOApicRegister) -> u32 {
        let val =
        unsafe {
            self.select_register(reg);
            read_volatile(self.offset_to_addr(4))
        }
        ; trace!("READ @{reg:?} => {val} aka 0x{val:x} aka 0b{val:b}"); val
    }

    fn write_u32(&mut self, reg: IOApicRegister, value: u32) {
        unsafe {
            self.select_register(reg);
            write_volatile(self.offset_to_addr(4), value)
        }
    }

    unsafe fn select_register(&self, reg: IOApicRegister) {
        let addr = self.offset_to_addr(0);
        write_volatile(addr, reg.as_u8() as _);
    }

    unsafe fn offset_to_addr(&self, offset: usize) -> *mut u32 {
        &(self.mapping.virtual_start().as_ref())[offset] as *const u32 as *mut u32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct IOApicRedirectionEntry {
    vector: u8,
    delivery_mode: DeliveryMode,
    destination_mode: DestinationMode,
    delivery_status: DeliveryStatus,
    polarity: InterruptPolarity,
    remote_irr: bool,
    trigger_mode: TriggerMode,
    mask: InterruptMask,
    destination: DestinationField,
}

#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum DeliveryMode {
    Fixed = 0b000,
    LowestPriority = 0b001,
    SystemManaged = 0b010,
    NMI = 0b100,
    INIT = 0b101,
    External = 0b111,
}

#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum DeliveryStatus {
    Idle = 0,
    SentPending = 1,
}

#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum DestinationMode {
    Physical = 0,
    Logical = 1,
}

#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum InterruptPolarity {
    HighActive = 0,
    LowActive = 1,
}

#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum TriggerMode {
    EdgeSensitive = 0,
    LevelSensitive = 1,
}

#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum InterruptMask {
    Unmasked = 0,
    Masked = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DestinationField {
    PhysicalApicId(u8),
    LogicalSetOfProcessors(u8),
}

impl DestinationField {
    #[must_use]
    pub const fn new(mode: DestinationMode, value: u8) -> Self {
        match mode {
            DestinationMode::Physical => Self::PhysicalApicId(value & 0b1111),
            DestinationMode::Logical => Self::LogicalSetOfProcessors(value),
        }
    }

    #[must_use]
    pub const fn as_u8(&self) -> u8 {
        match *self {
            Self::PhysicalApicId(val) => val,
            Self::LogicalSetOfProcessors(val) => val,
        }
    }
}


#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IOApicRegister {
    Id,
    Version,
    ArbitrationId,
    RedirectionEntry(u8),

    ManuallySpecified(u8),
}

impl IOApicRegister {
    pub const fn as_u8(&self) -> u8 {
        match *self {
            Self::Id => 0,
            Self::Version => 1,
            Self::ArbitrationId => 2,
            Self::RedirectionEntry(ent) => 0x10 + (ent * 2),
            Self::ManuallySpecified(ent) => ent,
        }
    }

    fn second_part_redir(&self) -> IOApicRegister {
        IOApicRegister::ManuallySpecified(self.as_u8() + 1)
    }
}

fn find_io_apic_base() -> Option<PhysAddr> {
    if let Some(madt) = ACPI_DATA.lock().madt.as_ref() {
        for entry in madt.entries() {
            trace!("  MADT entry: {entry:?}");

            if let MadtEntry::IoApic(apic) = entry {
                return Some(PhysAddr::new(apic.io_apic_address as _));
            }
        }
    }

    None
}
