// Copyright (C) 2024 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use crate::device::{
    pci::{ConfigurationSpaceMechanism, PciAddress},
    DeviceError,
    GenericDevice,
};

use super::NetworkDevice;

pub struct Intel8254xDevice {
    pci_addr: PciAddress,
}

impl GenericDevice for Intel8254xDevice {
    fn initialize(&mut self, pci: &impl ConfigurationSpaceMechanism) -> Result<(), DeviceError> {
        pci.enable_bus_mastering(self.pci_addr);

        let bar0 = pci.base_address(self.pci_addr, 0).expect("Should have BAR0");

        Ok(())
    }
}

impl NetworkDevice for Intel8254xDevice {

}
