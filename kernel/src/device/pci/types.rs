// Copyright (C) 2024 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use acpi::mcfg::McfgEntry;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd)]
pub struct PciAddress {
    pub segment: u16,
    pub bus: u8,
    pub device: u8,
    pub function: u8,
}

impl PciAddress {
    #[must_use]
    pub fn create_local_bus_address(&self, offset: u16, enabled: bool) -> u32 {
        let enabled = if enabled {
            0x80000000u32
        } else {
            0u32
        };

        ((self.bus as u32) << 16)
            | ((self.device as u32) << 11)
            | ((self.function as u32) << 8)
            | (offset as u32 & 0xFC)
            | (enabled)
    }

    #[must_use]
    pub fn create_express_offset(&self, offset: u16, bus_number_start: u8) -> u64 {
        ((self.bus as u64 - bus_number_start as u64) << 20)
                | (((self.device & 0b11111) as u64) << 15)
                | (((self.function & 0b111) as u64) << 12)
                + ((offset & 0xFFF) as u64)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PciBaseAddress(u32);

impl PciBaseAddress {
    #[must_use]
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn value(&self) -> u32 {
        self.0
    }

    #[must_use]
    pub const fn kind(&self) -> PciBaseAddressType {
        if self.0 & 0b1 == 1 {
            PciBaseAddressType::IOSpace
        } else {
            PciBaseAddressType::MemorySpace
        }
    }

    #[must_use]
    pub const fn actual_address(&self) -> u32 {
        match self.kind() {
            PciBaseAddressType::MemorySpace => self.value() & 0xFFF0,
            PciBaseAddressType::IOSpace => self.value() & 0xFFFFFFF0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PciBaseAddressType {
    MemorySpace,
    IOSpace,
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PciClassCode {
    Unclassified,
    MassStorageController,
    NetworkController,
    DisplayController,
    MultimediaController,
    MemoryController,
    Bridge,
    SimpleCommunicationController,
    BaseSystemPeripheral,
    InputDeviceController,
    DockingStation,
    Processor,
    SerialBusController,
    WirelessController,
    IntelligentController,
    SatelliteCommunicationController,
    EncryptionController,
    SignalProcessingController,
    ProcessingAccelerator,
    NonEssentialInstrumentation,

    Coprocessor,

    Unassigned,

    Reserved(u8),
}

impl PciClassCode {
    #[must_use]
    pub const fn new(id: u8) -> Self {
        match id {
            0x0 => Self::Unclassified,
            0x1 => Self::MassStorageController,
            0x2 => Self::NetworkController,
            0x3 => Self::DisplayController,
            0x4 => Self::MultimediaController,
            0x5 => Self::MemoryController,
            0x6 => Self::Bridge,
            0x7 => Self::SimpleCommunicationController,
            0x8 => Self::BaseSystemPeripheral,
            0x9 => Self::InputDeviceController,
            0xA => Self::DockingStation,
            0xB => Self::Processor,
            0xC => Self::SerialBusController,
            0xD => Self::WirelessController,
            0xE => Self::IntelligentController,
            0xF => Self::SatelliteCommunicationController,
            0x10 => Self::EncryptionController,
            0x11 => Self::SignalProcessingController,
            0x12 => Self::ProcessingAccelerator,
            0x13 => Self::NonEssentialInstrumentation,

            0x40 => Self::Coprocessor,

            0xFF => Self::Unassigned,

            _ => Self::Reserved(id),
        }
    }

    #[must_use]
    pub const fn value(&self) -> u8 {
        match self {
            Self::Unclassified => 0x0,
            Self::MassStorageController => 0x1,
            Self::NetworkController => 0x2,
            Self::DisplayController => 0x3,
            Self::MultimediaController => 0x4,
            Self::MemoryController => 0x5,
            Self::Bridge => 0x6,
            Self::SimpleCommunicationController => 0x7,
            Self::BaseSystemPeripheral => 0x8,
            Self::InputDeviceController => 0x9,
            Self::DockingStation => 0xA,
            Self::Processor => 0xB,
            Self::SerialBusController => 0xC,
            Self::WirelessController => 0xD,
            Self::IntelligentController => 0xE,
            Self::SatelliteCommunicationController => 0xF,
            Self::EncryptionController => 0x10,
            Self::SignalProcessingController => 0x11,
            Self::ProcessingAccelerator => 0x12,
            Self::NonEssentialInstrumentation => 0x13,

            Self::Coprocessor => 0x40,

            Self::Unassigned => 0xFF,

            Self::Reserved(id) => *id,
        }
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
pub enum PciHeaderType {
    Normal,
    PciToPciBridge,
    CardBusBridge,
    Unknown(u8),
}

impl PciHeaderType {
    #[must_use]
    pub const fn new(id: u8) -> Self {
        match id {
            0x0 => Self::Normal,
            0x1 => Self::PciToPciBridge,
            0x2 => Self::CardBusBridge,
            _ => Self::Unknown(id),
        }
    }

    #[must_use]
    pub const fn bar_count(&self) -> usize {
        match self {
            Self::Normal => 6,
            Self::PciToPciBridge => 2,
            Self::CardBusBridge => 0,
            Self::Unknown(..) => 0,
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

    pub const fn name(&self, class_code: PciClassCode) -> Option<&'static str> {
        match (class_code, self.value()) {
            (PciClassCode::Unassigned, 0x0) => Some("Non-VGA-Compatible"),
            (PciClassCode::Unassigned, 0x1) => Some("VGA-Compatible"),
            (PciClassCode::Unassigned, 0x80) => None,

            (PciClassCode::MassStorageController, 0x0) => Some("SCSI Bus Controller"),
            (PciClassCode::MassStorageController, 0x1) => Some("IDE Controller"),
            (PciClassCode::MassStorageController, 0x2) => Some("Floppy Disk Controller"),
            (PciClassCode::MassStorageController, 0x3) => Some("IPI Bus Controller"),
            (PciClassCode::MassStorageController, 0x4) => Some("RAID Controller"),
            (PciClassCode::MassStorageController, 0x5) => Some("ATA Controller"),
            (PciClassCode::MassStorageController, 0x6) => Some("Serial ATA Controller"),
            (PciClassCode::MassStorageController, 0x7) => Some("Serial Attached SCSI Controller"),
            (PciClassCode::MassStorageController, 0x8) => Some("Non-Volatile Memory Controller"),

            (PciClassCode::NetworkController, 0x0) => Some("Ethernet Controller"),
            (PciClassCode::NetworkController, 0x1) => Some("Token Ring Controller"),
            (PciClassCode::NetworkController, 0x2) => Some("FDDI Controller"),
            (PciClassCode::NetworkController, 0x3) => Some("ATM Controller"),
            (PciClassCode::NetworkController, 0x4) => Some("ISDN Controller"),
            (PciClassCode::NetworkController, 0x5) => Some("WorldFlip Controller"),
            (PciClassCode::NetworkController, 0x6) => Some("PICMG 2.14 Multi Computing Controller"),
            (PciClassCode::NetworkController, 0x7) => Some("InfiniBand Controller"),
            (PciClassCode::NetworkController, 0x8) => Some("Fabric Controller"),

            (PciClassCode::DisplayController, 0x0) => Some("VGA Compatible Controller"),
            (PciClassCode::DisplayController, 0x1) => Some("XGA Compatible Controller"),
            (PciClassCode::DisplayController, 0x2) => Some("3D Controller (Not VGA Compatible)"),

            (PciClassCode::MultimediaController, 0x0) => Some("Video Controller"),
            (PciClassCode::MultimediaController, 0x1) => Some("Audio Controller"),
            (PciClassCode::MultimediaController, 0x2) => Some("Computer Telephony Device"),
            (PciClassCode::MultimediaController, 0x3) => Some("Audio Device"),

            (PciClassCode::MemoryController, 0x0) => Some("RAM Controller"),
            (PciClassCode::MemoryController, 0x1) => Some("Flash Controller"),

            (PciClassCode::Bridge, 0x0) => Some("Host Bridge"),
            (PciClassCode::Bridge, 0x1) => Some("ISA Bridge"),
            (PciClassCode::Bridge, 0x2) => Some("EISA Bridge"),
            (PciClassCode::Bridge, 0x3) => Some("MCA Bridge"),
            (PciClassCode::Bridge, 0x4) => Some("PCI-to-PCI Bridge"),
            (PciClassCode::Bridge, 0x5) => Some("PCMCIA Bridge"),
            (PciClassCode::Bridge, 0x6) => Some("NuBus Bridge"),
            (PciClassCode::Bridge, 0x7) => Some("CardBus Bridge"),
            (PciClassCode::Bridge, 0x8) => Some("RACEway Bridge"),
            (PciClassCode::Bridge, 0x9) => Some("PCI-to-PCI Bridge (2)"),
            (PciClassCode::Bridge, 0xA) => Some("InfiniBand-to-PCI Bridge"),

            (PciClassCode::SimpleCommunicationController, 0x0) => Some("Serial Controller"),
            (PciClassCode::SimpleCommunicationController, 0x1) => Some("Parallel Controller"),
            (PciClassCode::SimpleCommunicationController, 0x2) => Some("Multiport Serial Controller"),
            (PciClassCode::SimpleCommunicationController, 0x3) => Some("Modem"),
            (PciClassCode::SimpleCommunicationController, 0x4) => Some("IEEE 488.1/2 (GPIB) Controller"),
            (PciClassCode::SimpleCommunicationController, 0x5) => Some("Smart Card Controller"),

            (PciClassCode::BaseSystemPeripheral, 0x0) => Some("PIC"),
            (PciClassCode::BaseSystemPeripheral, 0x1) => Some("DMA Controller"),
            (PciClassCode::BaseSystemPeripheral, 0x2) => Some("Timer"),
            (PciClassCode::BaseSystemPeripheral, 0x3) => Some("RTC Controller"),
            (PciClassCode::BaseSystemPeripheral, 0x4) => Some("PCI Hot-Plug Controller"),
            (PciClassCode::BaseSystemPeripheral, 0x5) => Some("SD Host Controller"),
            (PciClassCode::BaseSystemPeripheral, 0x6) => Some("IOMMU"),

            (PciClassCode::InputDeviceController, 0x0) => Some("Keyboard Controller"),
            (PciClassCode::InputDeviceController, 0x1) => Some("Digitizer Pen"),
            (PciClassCode::InputDeviceController, 0x2) => Some("Mouse Controller"),
            (PciClassCode::InputDeviceController, 0x3) => Some("Scanner Controller"),
            (PciClassCode::InputDeviceController, 0x4) => Some("Gameport Controller"),

            (PciClassCode::DockingStation, 0x0) => Some("Generic"),

            (PciClassCode::Processor, 0x0) => Some("i386"),
            (PciClassCode::Processor, 0x1) => Some("i486"),
            (PciClassCode::Processor, 0x2) => Some("Pentium"),
            (PciClassCode::Processor, 0x3) => Some("Pentium Pro"),
            (PciClassCode::Processor, 0x10) => Some("Alpha"),
            (PciClassCode::Processor, 0x20) => Some("PowerPC"),
            (PciClassCode::Processor, 0x30) => Some("MIPS"),
            (PciClassCode::Processor, 0x40) => Some("Co-Processor"),

            (PciClassCode::SerialBusController, 0x0) => Some("FireWire (IEEE 1394)"),
            (PciClassCode::SerialBusController, 0x1) => Some("ACCESS Bus"),
            (PciClassCode::SerialBusController, 0x2) => Some("SSA"),
            (PciClassCode::SerialBusController, 0x3) => Some("USB Controller"),
            (PciClassCode::SerialBusController, 0x4) => Some("Fibre Channel"),
            (PciClassCode::SerialBusController, 0x5) => Some("SMBus"),
            (PciClassCode::SerialBusController, 0x6) => Some("InfiniBand"),
            (PciClassCode::SerialBusController, 0x7) => Some("IPMI Interface"),
            (PciClassCode::SerialBusController, 0x8) => Some("SERCOS Interface (IEC 61491)"),
            (PciClassCode::SerialBusController, 0x9) => Some("CANbus Controller"),

            (PciClassCode::WirelessController, 0x0) => Some("iRDA Compatible"),
            (PciClassCode::WirelessController, 0x1) => Some("Consumer IR"),
            (PciClassCode::WirelessController, 0x10) => Some("RF"),
            (PciClassCode::WirelessController, 0x11) => Some("Bluetooth"),
            (PciClassCode::WirelessController, 0x12) => Some("Broadband"),
            (PciClassCode::WirelessController, 0x20) => Some("Ethernet (802.1a)"),
            (PciClassCode::WirelessController, 0x21) => Some("Ethernet (802.1b)"),

            (PciClassCode::IntelligentController, 0x0) => Some("I2O"),
            (PciClassCode::IntelligentController, 0x80) => None,

            (PciClassCode::SatelliteCommunicationController, 0x0) => Some("TV Controller"),
            (PciClassCode::SatelliteCommunicationController, 0x1) => Some("Audio Controller"),
            (PciClassCode::SatelliteCommunicationController, 0x2) => Some("Video Controller"),
            (PciClassCode::SatelliteCommunicationController, 0x3) => Some("Voice Controller"),
            (PciClassCode::SatelliteCommunicationController, 0x4) => Some("Data Controller"),
            (PciClassCode::SatelliteCommunicationController, 0x80) => None,

            (PciClassCode::EncryptionController, 0x0) => Some("Network and Computing Encrpytion/Decryption"),
            (PciClassCode::EncryptionController, 0x1) => Some("Entertainment Encryption/Decryption"),

            (PciClassCode::SignalProcessingController, 0x0) => Some("DPIO Modules"),
            (PciClassCode::SignalProcessingController, 0x1) => Some("Performance Counters"),
            (PciClassCode::SignalProcessingController, 0x10) => Some("Communication Synchronizer"),
            (PciClassCode::SignalProcessingController, 0x20) => Some("Signal Processing Management"),

            (_, 0x80) => Some("Other"),
            _ => None,
        }
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
