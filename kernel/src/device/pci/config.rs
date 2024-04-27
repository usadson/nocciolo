// Copyright (C) 2024 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use acpi::{mcfg::McfgEntry, AcpiHandler, PhysicalMapping};
use alloc::vec::Vec;
use lazy_static::lazy_static;
use log::warn;
use spin::Mutex;
use x86_64::instructions::port::{PortGeneric, ReadWriteAccess, WriteOnlyAccess};

use crate::device::acpi::NoccioloAcpiHandler;

use super::{PciAddress, PciClassCode, PciDeviceId, PciHeaderType, PciSubclass, PciVendorId};

pub const CONFIG_ADDRESS: u16 = 0xCF8;
pub const CONFIG_DATA: u16 = 0xCFC;

// Avoid racing conditions, since I/O ports must have exclusive access.
lazy_static! {
    static ref IO_PORTS: Mutex<PciIOPorts> = Mutex::new(PciIOPorts::new());
}

struct PciIOPorts {
    config_address_port: PortGeneric<u32, WriteOnlyAccess>,
    config_data_port: PortGeneric<u32, ReadWriteAccess>,
}

impl PciIOPorts {
    pub fn new() -> Self {
        Self {
            config_address_port: PortGeneric::new(CONFIG_ADDRESS),
            config_data_port: PortGeneric::new(CONFIG_DATA),
        }
    }
}

pub trait ConfigurationSpaceMechanism {
    fn read_word(&self, addr: PciAddress, offset: u16) -> u16;
    fn read_dword(&self, addr: PciAddress, offset: u16) -> u32;
    fn write_word(&self, addr: PciAddress, offset: u16, value: u16);

    fn vendor_id(&self, addr: PciAddress) -> PciVendorId {
        PciVendorId::new(self.read_word(addr, 0x0))
    }

    fn device_id(&self, addr: PciAddress) -> PciDeviceId {
        PciDeviceId::new(self.read_word(addr, 0x2))
    }

    fn command(&self, addr: PciAddress) -> u16 {
        self.read_word(addr, 0x4)
    }

    fn write_command(&self, addr: PciAddress, value: u16) {
        self.write_word(addr, 0x4, value);
    }

    fn status(&self, addr: PciAddress) -> u16 {
        self.read_word(addr, 0x6)
    }

    fn revision_id(&self, addr: PciAddress) -> u8 {
        (self.read_word(addr, 0x8) & 0xFF) as u8
    }

    fn prog_if(&self, addr: PciAddress) -> u8 {
        (self.read_word(addr, 0x8) >> 8) as u8
    }

    fn subclass(&self, addr: PciAddress) -> PciSubclass {
        PciSubclass::new((self.read_word(addr, 0xA) & 0xFF) as u8)
    }

    fn class_code(&self, addr: PciAddress) -> PciClassCode {
        PciClassCode::new((self.read_word(addr, 0xA) >> 8) as u8)
    }

    fn enumerate<'a>(&'a self) -> impl Iterator<Item = (PciAddress, PciVendorId, PciDeviceId)> + '_
            where Self: Sized {
        DeviceEnumerator {
            mechanism: self,
            device: 0,
            bus: 0,
        }
    }

    fn enable_bus_mastering(&self, addr: PciAddress) {
        let command = self.command(addr);
        self.write_command(addr, command | (1 << 2));
    }

    fn header_type(&self, addr: PciAddress) -> PciHeaderType {
        let ty = self.read_word(addr, 0xE) as u8;
        PciHeaderType::new(ty)
    }

    fn base_address(&self, addr: PciAddress, idx: usize) -> Option<u32> {
        if self.header_type(addr).bar_count() > idx {
            let idx = (idx * 4) as u16;
            Some(self.read_dword(addr, 0x10 + idx))
        } else {
            None
        }
    }
}

struct DeviceEnumerator<'a, Mechanism: ConfigurationSpaceMechanism> {
    mechanism: &'a Mechanism,
    bus: u16,
    device: u8,
}

impl<'a, Mechanism> Iterator for DeviceEnumerator<'a, Mechanism>
        where Mechanism: ConfigurationSpaceMechanism {
    type Item = (PciAddress, PciVendorId, PciDeviceId);

    fn next(&mut self) -> Option<Self::Item> {
        while self.bus < 256 {
            while self.device < 32 {
                let addr = PciAddress {
                    segment: 0,
                    bus: self.bus as u8,
                    device: self.device,
                    function: 0,
                };

                self.device += 1;

                let vendor_id = self.mechanism.vendor_id(addr);
                if vendor_id != PciVendorId::INVALID {
                    let device_id = self.mechanism.device_id(addr);
                    return Some((addr, vendor_id, device_id));
                }

            }

            self.bus += 1;
        }

        None
    }
}

pub struct PciExpressConfigurationSpace {
    mcfg_entries: Vec<McfgEntry>,
    mappings: Vec<PhysicalMapping<NoccioloAcpiHandler, u16>>,
}

impl PciExpressConfigurationSpace {
    pub fn new(entries: Vec<McfgEntry>) -> PciExpressConfigurationSpace {
        let mappings = entries.iter()
            .map(|entry| {
                let max = PciAddress {
                    segment: entry.pci_segment_group,
                    bus: entry.bus_number_end,
                    device: 255,
                    function: 255,
                };
                let size = max.create_express_offset(u16::MAX, entry.bus_number_start);
                log::trace!("MCFG entry of size 0x{size:X}");
                unsafe {
                    NoccioloAcpiHandler.map_physical_region(
                        entry.base_address as usize,
                        size as usize,
                    )
                }
            })
            .collect();

        Self {
            mcfg_entries: entries,
            mappings,
        }
    }

    fn config_space_to_address_space<T>(&self, addr: PciAddress, offset: u16) -> Option<*mut T> {
        for (idx, entry) in self.mcfg_entries.iter().enumerate() {
            if entry.pci_segment_group != addr.segment {
                continue;
            }

            if addr.bus < entry.bus_number_start || addr.bus > entry.bus_number_end {
                continue;
            }

            let addr = addr.create_express_offset(offset, entry.bus_number_start);
            let addr = self.mappings.get(idx)?.virtual_start().as_ptr() as u64 + addr;
            return Some(addr as *mut T);
        }

        warn!("Tried to read word outside PCI-E address space");
        None
    }

    fn read<T>(&self, addr: PciAddress, offset: u16) -> T
            where T: Copy + Default {
        let Some(addr) = self.config_space_to_address_space(addr, offset) else {
            return T::default();
        };
        unsafe { *addr }
    }
}

impl ConfigurationSpaceMechanism for PciExpressConfigurationSpace {
    fn read_word(&self, addr: PciAddress, offset: u16) -> u16 {
        self.read(addr, offset)
    }

    fn read_dword(&self, addr: PciAddress, offset: u16) -> u32 {
        self.read(addr, offset)
    }

    fn write_word(&self, addr: PciAddress, offset: u16, value: u16) {
        let Some(addr) = self.config_space_to_address_space(addr, offset) else {
            return;
        };

        unsafe {
            *addr = value;
        }
    }
}

pub struct PciLocalBusConfigurationSpace;

impl ConfigurationSpaceMechanism for PciLocalBusConfigurationSpace {
    fn read_word(&self, addr: PciAddress, offset: u16) -> u16 {
        let data = self.read_dword(addr, offset);
        let data = (data >> ((offset & 2) * 8)) & 0xFFFF;
        data as u16
    }

    fn read_dword(&self, addr: PciAddress, offset: u16) -> u32 {
        let mut ports = IO_PORTS.lock();

        let address = addr.create_local_bus_address(offset, true);
        unsafe {
            ports.config_address_port.write(address);
        }

        unsafe { ports.config_data_port.read() }
    }

    fn write_word(&self, addr: PciAddress, offset: u16, value: u16) {
        let mut ports = IO_PORTS.lock();

        let address = addr.create_local_bus_address(offset, false);
        unsafe {
            ports.config_address_port.write(address);
        }

        let data = unsafe { ports.config_data_port.read() };
        let value = value as u32;
        let data = if offset & 2 == 0 {
            value | (data & 0xFFFF0000)
        } else {
            (value << 16) | (data & 0xFFFF)
        };
        unsafe { ports.config_data_port.write(data) };
    }
}
