
use uart_16550::SerialPort;
use spin::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref SERIAL1: Mutex<SerialPort> = {
        let mut serial_port = unsafe { SerialPort::new(0x3F8) };
        serial_port.init();
        Mutex::new(serial_port)
    };
}

#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    interrupts::without_interrupts(|| {
        unsafe { SERIAL1.force_unlock() };
        SERIAL1.lock().write_fmt(args).expect("Printing to serial failed");
    });
}

pub fn print_in_interrupt(args: ::core::fmt::Arguments) {
    let mut port = unsafe { SerialPort::new(0x3F8) };
    port.init();

    use core::fmt::Write;
    _ = port.write_fmt(args);
}

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::serial::_print(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($($arg:tt)*) => ($crate::serial_print!("{}\n", format_args!($($arg)*)))
}

#[macro_export]
macro_rules! interrupt_print {
    ($($arg:tt)*) => {
        $crate::serial::print_in_interrupt(format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! interrupt_println {
    () => ($crate::interrupt_print!("\n"));
    ($($arg:tt)*) => ($crate::interrupt_print!("{}\n", format_args!($($arg)*)))
}
