// Copyright (C) 2023 - 2024 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

mod acpi;

use bootloader_api::BootInfo;


pub async fn init(boot_info: &'static BootInfo) {
    acpi::init(boot_info);
}