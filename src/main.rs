// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

#![no_main]
#![no_std]

mod debug;

use log::info;
use uefi::prelude::*;

#[entry]
fn main(_image_handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    // Initialize logger & memory services.
    uefi_services::init(&mut system_table).unwrap();

    info!("Nocciolo");

    debug::print_config_tables(&system_table);

    system_table.boot_services().stall(10_000_000);
    Status::SUCCESS
}
