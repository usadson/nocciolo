// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use log::info;
use uefi::{
    prelude::*,
    table::cfg::*,
};

pub fn print_config_tables(system_table: &SystemTable<Boot>) {
    for entry in system_table.config_table() {
        let name = match entry.guid {
            ACPI_GUID => Some("ACPI"),
            ACPI2_GUID => Some("ACPI2"),
            DEBUG_IMAGE_INFO_GUID => Some("DEBUG_IMAGE_INFO"),
            DXE_SERVICES_GUID => Some("DXE_SERVICES"),
            ESRT_GUID => Some("ESRT"),
            HAND_OFF_BLOCK_LIST_GUID => Some("HAND_OFF_BLOCK_LIST"),
            LZMA_COMPRESS_GUID => Some("LZMA_COMPRESS"),
            MEMORY_STATUS_CODE_RECORD_GUID => Some("MEMORY_STATUS_CODE_RECORD"),
            MEMORY_TYPE_INFORMATION_GUID => Some("MEMORY_TYPE_INFORMATION"),
            PROPERTIES_TABLE_GUID => Some("PROPERTIES_TABLE"),
            SMBIOS3_GUID => Some("SMBIOS3"),
            SMBIOS_GUID => Some("SMBIOS"),
            TIANO_COMPRESS_GUID => Some("TIANO_COMPRESS"),
            _ => None,
        };
        info!("Cte: {name:?}   {}", entry.guid);
    }
}
