// Copyright (C) 2024 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use bootloader_api::BootInfo;

pub mod symbols;

pub fn init(boot_info: &'static BootInfo) {
    self::symbols::init(boot_info);
}
