#![no_std]
#![no_main]
#![reexport_test_harness_main = "test_main"]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]
#![feature(const_mut_refs)]
#![feature(custom_test_frameworks)]
#![allow(internal_features)]
#![feature(lang_items)]
#![feature(allocator_api)]
#![test_runner(crate::test_runner)]

mod allocator;
mod device;
mod gdt;
mod interrupts;
mod memory;
mod meta;
mod serial;
mod task;
mod vga_text_buffer;
mod logging;

extern crate alloc;

use bootloader_api::{
    entry_point,
    BootInfo,
    config::{
        BootloaderConfig,
        Mapping,
    },
};

use x86_64::{instructions::interrupts::without_interrupts, VirtAddr};
use core::{panic::PanicInfo, time::Duration};
use log::{error, info, trace};

use crate::{device::pit, meta::System, task::{executor::Executor, keyboard, Task}};
use crate::vga_text_buffer::WRITER;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}

#[no_mangle]
pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[cfg(test)]
fn test_runner(tests: &[&dyn Fn()]) {
    println!("Running {} tests", tests.len());
    for test in tests {
        test();
    }
}

pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config.kernel_stack_size = 1024 * 1024;
    config
};


entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

#[no_mangle]
pub fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    serial_println!("----<[ nocciolo ]>----");
    init(boot_info);

    for i in (0..10).rev() {
        info!("See you in {i} seconds!");
        pit::sleep(Duration::from_secs(1));
    }

    System::request_shutdown();

    let mut executor = Executor::new();
    executor.spawn(Task::new(keyboard::print_keypresses()));
    executor.run();
}

#[no_mangle]
#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("[PANIC] {info}");
    hlt_loop();
}

fn init(boot_info: &'static BootInfo) {
    logging::init();

    if let Some(fb) = boot_info.framebuffer.as_ref() {
        WRITER.lock().set_fb(fb);
    }

    info!("----<[ nocciolo ]>----");

    gdt::init();
    interrupts::init_idt();

    trace!("Enabling Interrupts");

    trace!("Initializing the PIC");
    unsafe { interrupts::PICS.lock().initialize() };

    trace!("Initializing PIT");
    pit::init();

    trace!("Initializing Heap");
    init_heap(boot_info);

    trace!("Initializing ACPI");
    device::acpi::init(boot_info);

    if let Err(e) = interrupts::apic::init(boot_info) {
        trace!("Failed to initialize APIC: {e:?}");

        trace!("Initializing PICS");

    } else {
        unsafe { interrupts::PICS.lock().disable() };
    }

    x86_64::instructions::interrupts::enable();
    trace!("Interrupts enabled");

    // for i in (0..10).rev() {
    //     info!("See you in {i} seconds!");
    //     pit::sleep(Duration::from_secs(1));
    // }

    trace!("Initializing Kernel Runtime");
    meta::init(boot_info);

    trace!("Initializing Devices");
    device::init(boot_info);

    info!("Finished Initializing");
}

pub fn crash_test() {
    println!("Crashing...");
    let ptr = 0x0 as *mut u8;
    unsafe { ptr.write(0) };
    println!("Crashed!");
}

fn init_heap(boot_info: &'static BootInfo) {
    let physical_memory_offset;
    if let bootloader_api::info::Optional::Some(offset) = boot_info.physical_memory_offset {
        physical_memory_offset = offset;
    } else {
        panic!("No bootloader_api::BootInfo.physical_memory_offset");
    }

    let phys_mem_offset = VirtAddr::new(physical_memory_offset);

    unsafe {
        memory::init_mapper(phys_mem_offset);
        memory::init_frame_allocator(&boot_info.memory_regions);
    }

    memory::with_mapper(|mapper| memory::with_frame_allocator(|frame_allocator| {
        allocator::init_heap(mapper, frame_allocator)
            .expect("heap initialization failed");
    }));
}

#[lang = "eh_personality"]
#[no_mangle]
pub extern "C" fn eh_personality() {
    loop {}
}
