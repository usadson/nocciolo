use log::{LevelFilter, Metadata, Record};
use crate::serial_println;

static LOGGER: Logger = Logger{};

pub(super) fn init() {
    log::set_logger(&LOGGER)
        .expect("Failed to set logger");
    log::set_max_level(LevelFilter::Trace);
}

struct Logger;

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        _ = metadata;
        true
    }

    fn log(&self, record: &Record) {
        serial_println!("[{}] [{}] {}", record.metadata().target(), record.metadata().level(), record.args());
    }

    fn flush(&self) {
        // Nop
    }
}
