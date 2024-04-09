// Copyright (C) 2024 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd)]
pub struct PciAddress {
    pub segment: u16,
    pub bus: u8,
    pub device: u8,
    pub function: u8,
}

impl PciAddress {
    #[must_use]
    pub fn create_local_bus_address(&self, offset: u16) -> u32 {
        ((self.bus as u32) << 16)
            | ((self.device as u32) << 11)
            | ((self.function as u32) << 8)
            | (offset as u32 & 0xFC)
            | (0x80000000u32)
    }
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PciClassCode(u8);

impl PciClassCode {
    #[must_use]
    pub const fn new(id: u8) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn value(&self) -> u8 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PciDeviceId(u16);

impl PciDeviceId {
    #[must_use]
    pub const fn new(id: u16) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn value(&self) -> u16 {
        self.0
    }

    pub const fn name(&self, vendor_id: PciVendorId) -> Option<&'static str> {
        match vendor_id {
            PciVendorId::BOCHS => DeviceNames::get_bochs(self.0),
            PciVendorId::INTEL_CORPORATION => DeviceNames::get_intel(self.0),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PciSubclass(u8);

impl PciSubclass {
    #[must_use]
    pub const fn new(id: u8) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn value(&self) -> u8 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PciVendorId(u16);

impl PciVendorId {
    pub const INVALID: Self = Self(0xFFFF);

    pub const BOCHS: Self = Self(0x1234);
    pub const INTEL_CORPORATION: Self = Self(0x8086);

    #[must_use]
    pub const fn new(id: u16) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn value(&self) -> u16 {
        self.0
    }

    #[must_use]
    pub const fn name(&self) -> Option<&'static str> {
        match *self {
            Self::BOCHS => Some("Bochs"),
            Self::INTEL_CORPORATION => Some("Intel Corporation"),

            Self::INVALID => Some("INVALID"),

            _ => None,
        }
    }
}

struct DeviceNames;
impl DeviceNames {
    pub const fn get_bochs(id: u16) -> Option<&'static str> {
        match id {
            0x1111 => Some("Graphics Adapter"),
            _ => None,
        }
    }

    pub const fn get_intel(id: u16) -> Option<&'static str> {
        match id {
            0x1237 => Some("440FX - 82441FX PMC [Natoma]"),
            0x100e => Some("82540EM Gigabit Ethernet Controller"),
            0x7000 => Some("Propolis Virtual PIIX3 ISA Controller"),
            _ => None,
        }
    }
}
