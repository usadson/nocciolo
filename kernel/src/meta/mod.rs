// Copyright (C) 2024 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use bootloader_api::BootInfo;

mod console;
pub mod symbols;
mod system;

pub use self::console::Console;
pub use self::system::System;

pub fn init(boot_info: &'static BootInfo) {
    self::symbols::init(boot_info);
}
