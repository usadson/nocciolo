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

use bootloader_api::BootInfo;
use lazy_static::lazy_static;
use log::{trace, warn};

use spin::Mutex;
use x86_64::{
    registers::model_specific::Msr,
    PhysAddr,
};

use crate::{device::acpi::{
    NoccioloAcpiHandler,
    ACPI_DATA,
}, interrupts::PIC_1_OFFSET, logging::Colorize};

const IA32_APIC_BASE_MSR: u32 = 0x1B;

lazy_static! {
    static ref INSTANCE: Mutex<Option<LocalApic>> = Default::default();
}


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
    mapping: PhysicalMapping<NoccioloAcpiHandler, [u8; 0x800]>,
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
            NoccioloAcpiHandler.map_physical_region(addr.as_u64() as _, 0x800)
        };

        trace!("Local APIC is at {addr:?}");
        let this =

        Self {
            mapping
        }

        ;
        trace!("Which is mapped from 0x{:X}", unsafe { this.offset_to_addr(0) as usize });
        trace!("                  to 0x{:X}", this.get_mapped_end() as usize);
        this
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
        trace!("Reading from {register:?} ({:X}h)", register as usize);
        unsafe {
            read_volatile(self.register_to_addr(register))
        }
    }

    fn write(&mut self, register: LocalApicRegister, value: u32) {
        assert!(register.is_writable(), "Register {register:?} is {:?}", register.permissions());
        trace!("Writing to {register:?} ({:X}h) with value 0x{value:X}", register as usize);
        unsafe {
            let addr = self.register_to_addr(register) as *mut u32;
            write_volatile(addr, value)
        }
    }

    pub(super) unsafe fn register_to_addr(&self, register: LocalApicRegister) -> *mut u32 {
        let addr = self.offset_to_addr(register as usize);
        self.ensure_safe_addr(addr);
        trace!("  which is 0x{addr:p} addr ");
        addr
    }

    pub(super) unsafe fn offset_to_addr(&self, offset: usize) -> *mut u32 {
        ((&(self.mapping.virtual_start().as_ref())[offset]) as *const u8 as usize - 0x900) as *const u32 as *mut u32
    }

    pub fn publish(self) {
        let mut instance = INSTANCE.lock();
        *instance = Some(self);
    }

    fn ensure_safe_addr(&self, addr: *const u32) {
        debug_assert!(addr < self.get_mapped_end());
    }

    fn get_mapped_end(&self) -> *const u32 {
        let addr = unsafe {
            let addr = self.offset_to_addr(0);
            (addr as usize) + self.mapping.mapped_length()
        };
        addr as *const u32
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
#[allow(unused)]
pub enum LocalApicRegister {
    Id = 0x020,
    Version = 0x030,

    TaskPriority = 0x080,
    ArbitrationPriority = 0x090,
    ProcessorPriority = 0x0A0,
    EndOfInterrupt = 0x0B0,
    RemoteRead = 0x0C0,
    LogicalDestination = 0x0D0,
    DestinationFormat = 0x0E0,
    SpuriousInterruptVector = 0x0F0,

    ErrorStatus = 0x280,

    LvtCorrectedMachineCheckInterrupt = 0x2F0,

    InterruptCommand1 = 0x300,
    InterruptCommand2 = 0x310,

    LvtTimer = 0x320,
    LvtLint0 = 0x350,
    LvtLint1 = 0x360,
    LvtError = 0x370,
    InitialCount = 0x380,
    CurrentCount = 0x390,

    DivideConfiguration = 0x3E0,
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
