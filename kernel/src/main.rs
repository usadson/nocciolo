#![no_std]
#![no_main]
#![reexport_test_harness_main = "test_main"]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]
#![feature(const_mut_refs)]
#![feature(custom_test_frameworks)]
#![feature(lang_items)]
#![test_runner(crate::test_runner)]

mod allocator;
mod device;
mod gdt;
mod interrupts;
mod memory;
mod serial;
mod task;
mod vga_text_buffer;

extern crate alloc;

use bootloader_api::{
    entry_point,
    BootInfo,
    config::{
        BootloaderConfig,
        Mapping,
    },
};

use x86_64::VirtAddr;
use core::panic::PanicInfo;

use crate::{memory::BootInfoFrameAllocator, task::{executor::Executor, Task, keyboard}};
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
    config
};


entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

#[no_mangle]
pub fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    serial_println!("----<[ nocciolo ]>----");
    init(boot_info);

    serial_println!("FB: {:#?}", boot_info.framebuffer.as_ref().unwrap());

    println!("----<[ nocciolo ]>----");

    let mut executor = Executor::new();
    executor.spawn(Task::new(crate::device::init(boot_info)));
    executor.spawn(Task::new(keyboard::print_keypresses()));
    executor.run();

    panic!("end of _start() reached!");
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    interrupt_println!("[PANIC] {info}");
    unsafe { vga_text_buffer::WRITER.force_unlock() };
    println!("[PANIC] {}", info);
    hlt_loop();
}

fn init(boot_info: &'static BootInfo) {
    if let Some(fb) = boot_info.framebuffer.as_ref() {
        WRITER.lock().set_buffer(fb.buffer());
    }

    gdt::init();
    interrupts::init_idt();

    serial_println!("[init] Initializing PICS");
    unsafe { interrupts::PICS.lock().initialize() };

    serial_println!("[init] Enabling Interrupts");
    x86_64::instructions::interrupts::enable();

    serial_println!("[init] Initializing Heap");
    init_heap(boot_info);



    serial_println!("[init] Finished");
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
