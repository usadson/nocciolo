// Copyright (C) 2024 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use acpi::{address::{AddressSpace, GenericAddress}, AcpiError};
use aml::{AmlError, AmlName, AmlValue};
use log::{error, info, trace};
use raw_cpuid::CpuId;
use x86_64::instructions::port::Port;

use crate::device::acpi::{SystemState, ACPI_DATA};

/// Defined in ACPI section 7.1
const ACPI_SLP_EN: u16 = 1 << 13;

pub struct System;

impl System {
    pub fn request_shutdown() {
        let hypervisor = Self::detect_hypervisor();
        info!("Requesting shutdown (hypervisor={hypervisor:?})");
        match hypervisor {
            Some(HypervisorKind::Bochs) => unsafe {
                Port::new(0xB004).write(0x2000u16)
            }
            Some(HypervisorKind::QemuOld) => unsafe {
                Port::new(0x604).write(0x2000u16)
            }
            // Some(HypervisorKind::QemuNew) => unsafe {
            //     Port::new(0xf4).write(0x11u32)
            // }
            Some(HypervisorKind::VirtualBox) => unsafe {
                Port::new(0x4004).write(0x3400u16)
            }

            _ => shutdown_using_acpi().expect("Failed to shutdown using ACPI"),
        }
    }

    pub fn detect_hypervisor() -> Option<HypervisorKind> {
        let cpu: CpuId = CpuId::default();
        let cpu = cpu.get_processor_brand_string()?;
        trace!("CPU brand string: \"{}\"", cpu.as_str());
        match cpu.as_str() {
            "TCGTCGTCGTCG" => Some(HypervisorKind::QemuOld),
            "QEMU Virtual CPU version 2.5+" => Some(HypervisorKind::QemuNew),
            "VMwareVMware" => Some(HypervisorKind::VMWare),
            "VBoxVBoxVBox" => Some(HypervisorKind::VMWare),
            _ => None,
        }
    }
}

#[allow(unused)]
#[derive(Debug)]
enum AcpiShutdownErrorKind {
    Acpi(AcpiError),
    Aml(AmlError),

    NoAml,
    NoFadt,

    PmControlAddressNotInIoPortRange(u64),
    PmControlBlockNotInSystemIoSpace(AddressSpace),
    S5PathNotPackage,
    S5ValueNotInteger,
    S5ValueOutsideWordSize(u64),
}

impl From<AcpiError> for AcpiShutdownErrorKind {
    fn from(value: AcpiError) -> Self {
        Self::Acpi(value)
    }
}

impl From<AmlError> for AcpiShutdownErrorKind {
    fn from(value: AmlError) -> Self {
        Self::Aml(value)
    }
}

fn shutdown_using_acpi() -> Result<(), AcpiShutdownErrorKind> {
    trace!("Shutdown mechanism is ACPI");

    if let Err(err) = before_acpi_shutdown() {
        recover_acpi_shutdown();
        return Err(err);
    }

    if let Err(err) = do_shutdown_using_acpi() {
        recover_acpi_shutdown();
        return Err(err);
    }

    error!("We failed to sleep since you can see this");
    Ok(())
}

fn before_acpi_shutdown() -> Result<(), AcpiShutdownErrorKind> {
    let mut acpi = ACPI_DATA.lock();

    if let Some(aml) = acpi.aml.as_mut() {
        match aml.invoke_prepare_to_sleep(SystemState::S5) {
            Err(AmlError::ValueDoesNotExist(name)) => {
                // _PTS might not be present on some hardware (notably QEMU)
                if name.as_string() != "\\_PTS" {
                    return Err(AcpiShutdownErrorKind::Aml(AmlError::ValueDoesNotExist(name)));
                }
            }
            Err(e) => return Err(AcpiShutdownErrorKind::Aml(e)),
            _ => (),
        }
    }

    trace!("Invoked PrepareToSleep");
    Ok(())
}

/// If OSPM aborts the sleep state transition, OSPM should run the _WAK method
/// to indicate this condition to the platform.
fn recover_acpi_shutdown() {
    let Some(mut acpi) = ACPI_DATA.try_lock() else {
        return;
    };

    let Some(aml) = acpi.aml.as_mut() else {
        return;
    };

    trace!("Recovering from invalid Shutdown");
    _ = aml.invoke_system_wake(SystemState::S5);
}

fn do_shutdown_using_acpi() -> Result<(), AcpiShutdownErrorKind> {
    let acpi = ACPI_DATA.lock();

    let Some(aml) = acpi.aml.as_ref() else {
        return Err(AcpiShutdownErrorKind::NoAml);
    };

    let Some(fadt) = acpi.fadt.as_ref() else {
        return Err(AcpiShutdownErrorKind::NoFadt);
    };

    let s5_path = AmlName::from_str("\\_S5_")?;
    let s5_value = aml.namespace().get_by_path(&s5_path)?;
    let AmlValue::Package(s5_pkg) = s5_value else {
        error!("S5 value is not a package: {s5_value:#?}");
        return Err(AcpiShutdownErrorKind::S5PathNotPackage);
    };

    let pm1a_control_block = fadt.pm1a_control_block()?;
    perform_acpi_sleep(&s5_pkg[0], pm1a_control_block)?;

    if let Some(pm1b_control_block) = fadt.pm1b_control_block()? {
        perform_acpi_sleep(&s5_pkg[1], pm1b_control_block)?;
    }

    Ok(())
}

fn perform_acpi_sleep(s5_value: &AmlValue, control_block: GenericAddress) -> Result<(), AcpiShutdownErrorKind> {
    let AmlValue::Integer(sleep_type) = s5_value else {
        return Err(AcpiShutdownErrorKind::S5ValueNotInteger);
    };

    let sleep_type = *sleep_type;
    if sleep_type > u16::MAX as u64 {
        return Err(AcpiShutdownErrorKind::S5ValueOutsideWordSize(sleep_type));
    }

    let sleep_type = sleep_type as u16;

    if control_block.address_space != AddressSpace::SystemIo {
        error!("PM control block not in System I/O Address Space: {control_block:#x?}");
        return Err(AcpiShutdownErrorKind::PmControlBlockNotInSystemIoSpace(control_block.address_space));
    }

    if control_block.address > u16::MAX as u64 {
        return Err(AcpiShutdownErrorKind::PmControlAddressNotInIoPortRange(control_block.address));
    }

    unsafe {
        Port::new(control_block.address as _).write(ACPI_SLP_EN | sleep_type);
    }

    Ok(())
}

#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HypervisorKind {
    Bochs,
    QemuOld,
    QemuNew,
    VMWare,
    VirtualBox,
}
