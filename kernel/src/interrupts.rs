use x86_64::structures::idt::{
    InterruptDescriptorTable,
    InterruptStackFrame,
    PageFaultErrorCode,
};

use pic8259::ChainedPics;
use lazy_static::lazy_static;
use log::trace;

use crate::{hlt_loop, interrupt_println, print, vga_text_buffer};

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }
}

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.double_fault.set_handler_fn(double_fault_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);

        idt[InterruptIndex::Timer.as_u8()].set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_u8()].set_handler_fn(keyboard_interrupt_handler);

        idt
    };
}
pub static PICS: spin::Mutex<ChainedPics> = spin::Mutex::new(
    unsafe {
        ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET)
    }
);

pub fn init_idt() {
    trace!("Loading IDT");
    IDT.load();
    trace!("Loaded IDT");
}

#[inline(always)]
fn interrupt_begin() {
    interrupt_println!("Interrupt Begin");
    unsafe { vga_text_buffer::WRITER.force_unlock() };
}

//
// CPU Interrupts
//

#[no_mangle]
extern "x86-interrupt"
fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    interrupt_begin();
    interrupt_println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

#[no_mangle]
extern "x86-interrupt"
fn double_fault_handler(stack_frame: InterruptStackFrame, _error_code: u64) -> ! {
    interrupt_begin();
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

#[no_mangle]
extern "x86-interrupt"
fn page_fault_handler(stack_frame: InterruptStackFrame, error_code: PageFaultErrorCode) {
    use x86_64::registers::control::Cr2;

    interrupt_begin();
    interrupt_println!("EXCEPTION: PAGE FAULT");
    interrupt_println!("Accessed Address: {:?}", Cr2::read());
    interrupt_println!("Error Code: {:?}", error_code);
    interrupt_println!("{:#?}", stack_frame);
    hlt_loop();
}

//
// Hardware Interrupts
//

#[no_mangle]
extern "x86-interrupt"
fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;

    interrupt_begin();

    let mut port = Port::new(0x60);

    let scancode: u8 = unsafe { port.read() };
    crate::task::keyboard::add_scancode(scancode);

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

#[no_mangle]
extern "x86-interrupt"
fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    print!(".");

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}
