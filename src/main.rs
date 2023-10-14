#![no_main]
#![no_std]

use log::info;
use uefi::prelude::*;

#[entry]
fn main(_image_handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    // Initialize logger & memory services.
    uefi_services::init(&mut system_table).unwrap();

    info!("Nocciolo");

    system_table.boot_services().stall(10_000_000);
    Status::SUCCESS
}
