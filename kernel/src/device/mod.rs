// Copyright (C) 2023 - 2024 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

mod acpi;

use ::acpi::AcpiError;
use aml::AmlError;
use bootloader_api::BootInfo;

pub async fn init(boot_info: &'static BootInfo) {
    acpi::init(boot_info);
}

#[derive(Debug)]
pub struct DeviceError {
    kind: DeviceErrorKind,
    region: &'static str,
}

impl DeviceError {
    pub fn with_region(mut self, region: &'static str) -> Self {
        Self {
            region,
            ..self
        }
    }

    pub fn acpi(error: AcpiError) -> Self {
        DeviceError {
            kind: DeviceErrorKind::Acpi(error),
            region: "(unknown)",
        }
    }

    pub fn aml(error: AmlError) -> Self {
        DeviceError {
            kind: DeviceErrorKind::Aml(error),
            region: "(unknown)",
        }
    }
}

#[derive(Debug)]
pub enum DeviceErrorKind {
    Acpi(AcpiError),
    Aml(AmlError),
}

impl From<AcpiError> for DeviceError {
    fn from(value: AcpiError) -> Self {
        Self::acpi(value)
    }
}

impl From<AmlError> for DeviceError {
    fn from(value: AmlError) -> Self {
        Self::aml(value)
    }
}

