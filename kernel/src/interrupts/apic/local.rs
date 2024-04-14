// Copyright (C) 2024 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use core::{
    fmt::Debug,
    ptr::{
        read_volatile,
        write_volatile,
    },
};

use acpi::{
    madt::MadtEntry,
    AcpiHandler,
    PhysicalMapping,
};

use bootloader_api::{info::MemoryRegionKind, BootInfo};
use log::{trace, warn};

use x86_64::{
    registers::model_specific::Msr,
    PhysAddr,
};

use crate::{device::acpi::{
    NoccioloAcpiHandler,
    ACPI_DATA,
}, interrupts::PIC_1_OFFSET, logging::Colorize};

const IA32_APIC_BASE_MSR: u32 = 0x1B;

fn find_local_apic_base() -> PhysAddr {
    if let Some(madt) = ACPI_DATA.lock().madt.as_ref() {
        for entry in madt.entries() {
            trace!("  MADT entry: {entry:?}");

            if let MadtEntry::LocalApicAddressOverride(entry) = entry {
                let addr = entry.local_apic_address;
                trace!("Local APIC base was overridden (MADT entry): {addr}");
                return PhysAddr::new(addr);
            }
        }
    }

    // The model-specific register contains the base
    let apic_base_msr = Msr::new(IA32_APIC_BASE_MSR);
    let apic_base = unsafe { apic_base_msr.read() };
    trace!("Local APIC base was found in the MSR: {:x}", apic_base);
    PhysAddr::new(apic_base)
}

fn set_local_apic_base(addr: PhysAddr) {
    let mut apic_base_msr = Msr::new(IA32_APIC_BASE_MSR);
    unsafe {
        apic_base_msr.write(addr.as_u64());
    }
}

pub struct LocalApic {
    mapping: PhysicalMapping<NoccioloAcpiHandler, [u32; 256]>,
}

impl LocalApic {
    #[must_use]
    pub fn new(boot_info: &BootInfo) -> Self {
        let addr = find_local_apic_base();
        verify_in_correct_region(addr, boot_info);
        Self::from_addr(addr)
    }

    #[must_use]
    pub fn from_addr(addr: PhysAddr) -> Self {
        // Section 11.4.1 of 3rd volume of Intel SDM recommends mapping the base
        // address page as strong uncacheable for correct APIC operation.
        set_local_apic_base(addr);

        let mapping = unsafe {
            NoccioloAcpiHandler.map_physical_region(addr.as_u64() as _, 0x400)
        };

        Self {
            mapping
        }
    }

    pub fn initialize(&mut self) {
        self.enable();

        // Set the LVT Timer interrupt index to our expected interrupt
        self.write(LocalApicRegister::LvtTimer, PIC_1_OFFSET as _);

        self.set_timer_initial_counter(0);
        // sleep :(
        let count = self.current_count();
        // adjust
        // divide

    }

    fn enable(&mut self) {
        // let vector = self.read(LocalApicOffset::SpuriousInterruptVector);
        // let vector = vector | 0x100;
        let vector = 0xFF;
        self.write(LocalApicRegister::SpuriousInterruptVector, vector);
    }

    pub fn do_test_stuff(&mut self) {
        trace!("Timer LVT is set to: 0x{:x}", self.read(LocalApicRegister::LvtTimer));
        trace!("LINT0 is set to: 0x{:x}", self.read(LocalApicRegister::LvtLint0));
        trace!("LINT1 is set to: 0x{:x}", self.read(LocalApicRegister::LvtLint1));
    }

    pub fn id(&self) -> u32 {
        self.read(LocalApicRegister::Id)
    }

    fn set_timer_divide(&mut self, divide: u32) {
        self.write(LocalApicRegister::DivideConfiguration, divide);
    }

    pub fn set_timer_initial_counter(&mut self, counter: u32) {
        self.write(LocalApicRegister::InitialCount, counter);
    }

    pub fn stop_timer(&mut self) {
        let reg = LocalVectorTableRegister::new_masked_timer();
        self.write(LocalApicRegister::LvtTimer, reg.as_u32());
    }

    pub fn current_count(&self) -> u32 {
        self.read(LocalApicRegister::CurrentCount)
    }

    pub fn version(&self) -> u32 {
        self.read(LocalApicRegister::Version)
    }

    fn read(&self, register: LocalApicRegister) -> u32 {
        assert!(register.is_readable(), "Register {register:?} is {:?}", register.permissions());
        unsafe {
            read_volatile(self.register_to_addr(register))
        }
    }

    fn write(&mut self, register: LocalApicRegister, value: u32) {
        assert!(register.is_writable(), "Register {register:?} is {:?}", register.permissions());
        unsafe {
            let addr = self.register_to_addr(register) as *mut u32;
            write_volatile(addr, value)
        }
    }

    unsafe fn register_to_addr(&self, register: LocalApicRegister) -> *const u32 {
        self.offset_to_addr((register as usize) * 0x4)
    }

    unsafe fn offset_to_addr(&self, offset: usize) -> *const u32 {
        &(self.mapping.virtual_start().as_ref())[offset] as *const u32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[allow(unused)]
pub enum LocalApicRegister {
    #[deprecated]
    Reserved0,

    Id,
    Version,

    #[deprecated]
    Reserved1,
    #[deprecated]
    Reserved2,
    #[deprecated]
    Reserved3,
    #[deprecated]
    Reserved4,
    #[deprecated]
    Reserved5,

    TaskPriority,
    ArbitrationPriority,
    ProcessorPriority,
    EndOfInterrupt,
    RemoteRead,
    LogicalDestination,
    DestinationFormat,
    SpuriousInterruptVector,

    ErrorStatus = 0x28,

    LvtCorrectedMachineCheckInterrupt = 0x2F,

    InterruptCommand1 = 0x30,
    InterruptCommand2 = 0x31,

    LvtTimer = 0x32,
    LvtLint0 = 0x35,
    LvtLint1 = 0x36,
    LvtError = 0x37,
    InitialCount = 0x38,
    CurrentCount = 0x39,

    DivideConfiguration = 0x3E,
}

impl LocalApicRegister {
    pub const fn permissions(&self) -> ApicRegisterPermissions {
        match self {
            // Actually R/W, but the Intel specification discourages writing
            Self::Id => ApicRegisterPermissions::ReadOnly,
            Self::Version => ApicRegisterPermissions::ReadOnly,
            Self::TaskPriority => ApicRegisterPermissions::ReadWrite,
            Self::ArbitrationPriority => ApicRegisterPermissions::ReadOnly,
            Self::ProcessorPriority => ApicRegisterPermissions::ReadOnly,
            Self::EndOfInterrupt => ApicRegisterPermissions::WriteOnly,
            Self::RemoteRead => ApicRegisterPermissions::ReadOnly,
            Self::LogicalDestination => ApicRegisterPermissions::ReadWrite,
            Self::DestinationFormat => ApicRegisterPermissions::ReadWrite,
            Self::SpuriousInterruptVector => ApicRegisterPermissions::ReadWrite,
            Self::ErrorStatus => ApicRegisterPermissions::ReadOnly,
            Self::LvtCorrectedMachineCheckInterrupt => ApicRegisterPermissions::ReadWrite,
            Self::InterruptCommand1 => ApicRegisterPermissions::ReadWrite,
            Self::InterruptCommand2 => ApicRegisterPermissions::ReadWrite,
            Self::LvtTimer => ApicRegisterPermissions::ReadWrite,
            Self::LvtLint0 => ApicRegisterPermissions::ReadWrite,
            Self::LvtLint1 => ApicRegisterPermissions::ReadWrite,
            Self::LvtError => ApicRegisterPermissions::ReadWrite,
            Self::InitialCount => ApicRegisterPermissions::ReadWrite,
            Self::CurrentCount => ApicRegisterPermissions::ReadOnly,
            Self::DivideConfiguration => ApicRegisterPermissions::ReadWrite,
            _ => ApicRegisterPermissions::None,
        }
    }

    pub const fn is_readable(&self) -> bool {
        match self.permissions() {
            ApicRegisterPermissions::ReadOnly => true,
            ApicRegisterPermissions::ReadWrite => true,

            ApicRegisterPermissions::None => false,
            ApicRegisterPermissions::WriteOnly => false,
        }
    }

    pub const fn is_writable(&self) -> bool {
        match self.permissions() {
            ApicRegisterPermissions::WriteOnly => true,
            ApicRegisterPermissions::ReadWrite => true,

            ApicRegisterPermissions::None => false,
            ApicRegisterPermissions::ReadOnly => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApicRegisterPermissions {
    None,
    ReadWrite,
    ReadOnly,
    WriteOnly,
}

struct LocalVectorTableRegister {
    vector: u8,
    delivery_mode: VectorDeliveryMode,
    delivery_status: VectorDeliverStatus,
    is_low_triggered: bool,
    is_remote_irr: bool,
    trigger_mode: VectorTriggerMode,
    is_masked: bool,
    timer_mode: VectorTimerMode,
}

impl LocalVectorTableRegister {
    pub fn new_timer(vector: u8, status: VectorDeliverStatus, is_masked: bool, mode: VectorTimerMode) -> Self {
        Self {
            vector,
            delivery_mode: VectorDeliveryMode::NMI,
            delivery_status: status,
            is_masked,
            timer_mode: mode,

            // Reserved:
            is_low_triggered: false,
            is_remote_irr: false,
            trigger_mode: VectorTriggerMode::Edge,
        }
    }

    pub fn new_masked_timer() -> Self {
        Self::new_timer(0, VectorDeliverStatus::Idle, true, VectorTimerMode::Periodic)
    }

    pub fn as_u32(&self) -> u32 {
        let reserved = 0;
        (self.vector as u32 & 0b1111_1111)
            | ((self.delivery_mode as u32 & 0b111) << 8)
            | ((reserved as u32 & 0b1) << 11)
            | ((self.delivery_status as u32 & 0b1) << 12)
            | ((self.is_low_triggered as u32 & 0b1) << 13)
            | ((self.is_remote_irr as u32 & 0b1) << 14)
            | ((self.trigger_mode as u32 & 0b1) << 15)
            | ((self.is_masked as u32 & 0b1) << 16)
            | ((self.timer_mode as u32 & 0b11) << 17)
    }
}

#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum VectorDeliveryMode {
    Fixed = 0b000,
    SMI = 0b010,
    NMI = 0b100,

    ExtInt = 0b11,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum VectorDeliverStatus {
    Idle = 0,
    SendPending = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum VectorTriggerMode {
    Edge = 0,
    Level = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum VectorTimerMode {
    OneShot = 0b00,
    Periodic = 0b01,
    TscDeadline = 0b10,
}

fn verify_in_correct_region(addr: PhysAddr, boot_info: &BootInfo) {
    let addr = addr.as_u64();

    for region in boot_info.memory_regions.iter() {
        if addr >= region.start && addr <= region.end {
            trace!("APIC is in region: {region:#?}");
            return;
        }
    }

    warn!("APIC address {:X} was not found in any known MemoryRegion!", addr.red());
}
