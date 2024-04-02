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

use bootloader::{entry_point, BootInfo};
use x86_64::VirtAddr;
use core::panic::PanicInfo;

use crate::{memory::BootInfoFrameAllocator, task::{executor::Executor, Task, keyboard}};

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

entry_point!(kernel_main);

pub fn kernel_main(boot_info: &'static BootInfo) -> ! {
    println!("----<[ nocciolo ]>----");

    init(boot_info);

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
    println!("[PANIC] {}", info);
    hlt_loop();
}

fn init(boot_info: &'static BootInfo) {
    gdt::init();
    interrupts::init_idt();

    unsafe { interrupts::PICS.lock().initialize() };
    x86_64::instructions::interrupts::enable();

    init_heap(boot_info);
}

fn init_heap(boot_info: &'static BootInfo) {
    let physical_memory_offset = boot_info.physical_memory_offset;

    let phys_mem_offset = VirtAddr::new(physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe {
        BootInfoFrameAllocator::init(&boot_info.memory_map)
    };

    allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("heap initialization failed");
}

#[lang = "eh_personality"]
#[no_mangle]
pub extern "C" fn eh_personality() {
    loop {}
}
