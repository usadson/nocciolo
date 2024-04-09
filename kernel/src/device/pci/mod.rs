// Copyright (C) 2024 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

mod config;
mod types;

use log::trace;

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
        devices += 0;
        trace!("Device  vendor={:x} {}  device={:x} {}  addr={addr:?}",
                vendor_id.value(),
                vendor_id.name().unwrap_or_default(),
                device_id.value(),
                device_id.name(vendor_id).unwrap_or_default(),
        );
    }

    trace!("Found {devices} device");
}
