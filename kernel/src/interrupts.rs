use x86_64::structures::idt::{
    InterruptDescriptorTable,
    InterruptStackFrame,
    PageFaultErrorCode,
};

use pic8259::ChainedPics;
use lazy_static::lazy_static;
use log::trace;

use crate::{hlt_loop, interrupt_println, meta::symbols, print, vga_text_buffer};

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
        idt.divide_error.set_handler_fn(division_error_handler);
        idt.debug.set_handler_fn(debug_handler);
        idt.non_maskable_interrupt.set_handler_fn(non_maskable_interrupt_handler);
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.overflow.set_handler_fn(overflow_handler);
        idt.bound_range_exceeded.set_handler_fn(bound_range_exceeded_handler);
        idt.invalid_opcode.set_handler_fn(invalid_opcode_handler);
        idt.device_not_available.set_handler_fn(device_not_available_handler);
        idt.double_fault.set_handler_fn(double_fault_handler);

        idt.invalid_tss.set_handler_fn(invalid_tss_handler);
        idt.segment_not_present.set_handler_fn(segment_not_present_handler);
        idt.stack_segment_fault.set_handler_fn(stack_segment_fault_handler);
        idt.general_protection_fault.set_handler_fn(general_protection_fault_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);

        // TODO: x87

        idt.alignment_check.set_handler_fn(alignment_check_handler);
        idt.machine_check.set_handler_fn(machine_check_handler);
        idt.simd_floating_point.set_handler_fn(simd_floating_point_exception_handler);
        idt.virtualization.set_handler_fn(virtualization_exception_handler);
        idt.cp_protection_exception.set_handler_fn(control_protection_exception_handler);

        idt.hv_injection_exception.set_handler_fn(hypervisor_injection_exception);
        idt.vmm_communication_exception.set_handler_fn(vmm_communication_exception_handler);
        idt.security_exception.set_handler_fn(security_exception_handler);

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
    panic!("EXCEPTION: DOUBLE FAULT ({_error_code:X})\n{:#?}", stack_frame);
    hlt_loop();
}

#[no_mangle]
extern "x86-interrupt"
fn page_fault_handler(stack_frame: InterruptStackFrame, error_code: PageFaultErrorCode) {
    use x86_64::registers::control::Cr2;

    interrupt_begin();
    interrupt_println!("EXCEPTION: PAGE FAULT");
    interrupt_println!("Accessed Address: {:?}", Cr2::read());
    interrupt_println!("Error Code: {:?}", error_code);
    interrupt_println!("Function: {:?}", symbols::resolve(stack_frame.stack_pointer.as_u64()));
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
    // Do some stuff here

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

#[no_mangle]
extern "x86-interrupt"
fn division_error_handler(stack_frame: InterruptStackFrame) {
    interrupt_begin();
    interrupt_println!("EXCEPTION: DIVISION ERROR\n{:#?}", stack_frame);
}

#[no_mangle]
extern "x86-interrupt"
fn non_maskable_interrupt_handler(stack_frame: InterruptStackFrame) {
    interrupt_begin();
    interrupt_println!("EXCEPTION: NMI\n{:#?}", stack_frame);
}

#[no_mangle]
extern "x86-interrupt"
fn debug_handler(stack_frame: InterruptStackFrame) {
    interrupt_begin();
    interrupt_println!("EXCEPTION: DEBUG\n{:#?}", stack_frame);
}

#[no_mangle]
extern "x86-interrupt"
fn overflow_handler(stack_frame: InterruptStackFrame) {
    interrupt_begin();
    interrupt_println!("EXCEPTION: OVERFLOW\n{:#?}", stack_frame);
}

#[no_mangle]
extern "x86-interrupt"
fn bound_range_exceeded_handler(stack_frame: InterruptStackFrame) {
    interrupt_begin();
    interrupt_println!("EXCEPTION: BOUND RANGE EXCEEDED\n{:#?}", stack_frame);
}

#[no_mangle]
extern "x86-interrupt"
fn invalid_opcode_handler(stack_frame: InterruptStackFrame) {
    interrupt_begin();
    interrupt_println!("EXCEPTION: INVALID OPCODE\n{:#?}", stack_frame);
}

#[no_mangle]
extern "x86-interrupt"
fn device_not_available_handler(stack_frame: InterruptStackFrame) {
    interrupt_begin();
    interrupt_println!("EXCEPTION: DEVICE NOT AVAILABLE\n{:#?}", stack_frame);
}

#[no_mangle]
extern "x86-interrupt"
fn invalid_tss_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    interrupt_begin();
    interrupt_println!("EXCEPTION: INVALID TSS ({error_code}) \n{:#?}", stack_frame);
}

#[no_mangle]
extern "x86-interrupt"
fn segment_not_present_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    interrupt_begin();
    interrupt_println!("EXCEPTION: SEGMENT NOT PRESENT ({error_code}) \n{:#?}", stack_frame);
}

#[no_mangle]
extern "x86-interrupt"
fn stack_segment_fault_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    interrupt_begin();
    interrupt_println!("EXCEPTION: STACK SEGMENT FAULT ({error_code}) \n{:#?}", stack_frame);
}

#[no_mangle]
extern "x86-interrupt"
fn general_protection_fault_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    interrupt_begin();
    interrupt_println!("EXCEPTION: GENERAL PROTECTION FAULT ({error_code}) \n{:#?}", stack_frame);

    hlt_loop();
}

#[no_mangle]
extern "x86-interrupt"
fn alignment_check_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    interrupt_begin();
    interrupt_println!("EXCEPTION: ALIGNMENT CHECK ({error_code}) \n{:#?}", stack_frame);
}

#[no_mangle]
extern "x86-interrupt"
fn machine_check_handler(stack_frame: InterruptStackFrame) -> ! {
    interrupt_begin();
    interrupt_println!("EXCEPTION: MACHINE CHECK\n{:#?}", stack_frame);
    panic!("MACHINE CHECK");
}

#[no_mangle]
extern "x86-interrupt"
fn simd_floating_point_exception_handler(stack_frame: InterruptStackFrame) {
    interrupt_begin();
    interrupt_println!("EXCEPTION: SIMD FLOATING POINT EXCEPTION\n{:#?}", stack_frame);
}

#[no_mangle]
extern "x86-interrupt"
fn control_protection_exception_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    interrupt_begin();
    interrupt_println!("EXCEPTION: CONTROL PROTECTION EXCEPTION ({error_code})\n{:#?}", stack_frame);
}

#[no_mangle]
extern "x86-interrupt"
fn virtualization_exception_handler(stack_frame: InterruptStackFrame) {
    interrupt_begin();
    interrupt_println!("EXCEPTION: VIRTUALIZATION EXCEPTION)\n{:#?}", stack_frame);
}

#[no_mangle]
extern "x86-interrupt"
fn hypervisor_injection_exception(stack_frame: InterruptStackFrame) {
    interrupt_begin();
    interrupt_println!("EXCEPTION: HYPERVISOR INJECTION EXCEPTION\n{:#?}", stack_frame);
}

#[no_mangle]
extern "x86-interrupt"
fn vmm_communication_exception_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    interrupt_begin();
    interrupt_println!("EXCEPTION: VMM COMMUNICATION EXCEPTION ({error_code})\n{:#?}", stack_frame);
}

#[no_mangle]
extern "x86-interrupt"
fn security_exception_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    interrupt_begin();
    interrupt_println!("EXCEPTION: SECURITY EXCEPTION ({error_code})\n{:#?}", stack_frame);
}
