// Copyright (C) 2024 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

mod config;
mod types;

use log::{info, trace};

pub use self::{
    config::{
        ConfigurationSpaceMechanism,
        PciLocalBusConfigurationSpace,
    },
    types::{
        PciAddress,
        PciClassCode,
        PciDeviceId,
        PciSubclass,
        PciVendorId,
    },
};

pub(super) fn init(boot_info: &bootloader_api::BootInfo) {
    _ = boot_info;

    let mechanism = PciLocalBusConfigurationSpace;
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
