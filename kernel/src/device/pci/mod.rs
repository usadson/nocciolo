// Copyright (C) 2024 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

mod config;
mod types;

use log::{info, trace};

use self::config::PciExpressConfigurationSpace;
pub use self::{
    config::{
        ConfigurationSpaceMechanism,
        PciLocalBusConfigurationSpace,
    },
    types::{
        PciAddress,
        PciClassCode,
        PciDeviceId,
        PciHeaderType,
        PciSubclass,
        PciVendorId,
    },
};

use super::acpi::ACPI_DATA;

pub(super) fn init(boot_info: &bootloader_api::BootInfo) {
    _ = boot_info;

    if let Some(pci_express) = try_create_pci_express_mechanism() {
        init_using(&pci_express);
    } else {
        init_using(&PciLocalBusConfigurationSpace);
    }
}

fn init_using(mechanism: &impl ConfigurationSpaceMechanism) {
    trace!("Enumerating devices...");

    let mut devices = 0;
    for (addr, vendor_id, device_id) in mechanism.enumerate() {
        devices += 1;
        info!("Device vendor={:x} device={:x} addr={addr:?}",
                vendor_id.value(),
                device_id.value(),
        );

        let class = mechanism.class_code(addr);
        let subclass = mechanism.subclass(addr);
        info!("  Class: {class:?}, subclass 0x{:x} {}", subclass.value(), subclass.name(class).unwrap_or_default());

        if let Some(vendor_name) = vendor_id.name() {
            info!("  Name: {vendor_name}     {}", device_id.name(vendor_id).unwrap_or_default());
        }
    }

    info!("Found {devices} PCI devices");
}

fn try_create_pci_express_mechanism() -> Option<PciExpressConfigurationSpace> {
    let acpi = ACPI_DATA.lock();
    let mcfg_entries = acpi.mcfg.as_ref()?.entries().to_vec();

    Some(PciExpressConfigurationSpace::new(mcfg_entries))
}
