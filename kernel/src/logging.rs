use core::fmt::{Display, Formatter, Write};
use log::{LevelFilter, Metadata, Record};
use crate::serial_println;

static LOGGER: Logger = Logger{};

pub(super) fn init() {
    log::set_logger(&LOGGER)
        .expect("Failed to set logger");
    log::set_max_level(LevelFilter::Trace);
}

struct Logger;

pub struct Colored<S> {
    color: Color,
    inner: S,
}

pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    Default,
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        _ = metadata;
        true
    }

    fn log(&self, record: &Record) {
        serial_println!("[{}] [\x1b[31m{}\x1b[0m] {}", record.metadata().target().white(), record.metadata().level().stylized(), record.args());
    }

    fn flush(&self) {
        // Nop
    }
}

impl Color {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Black => "30",
            Self::Red => "31",
            Self::Green => "32",
            Self::Yellow => "33",
            Self::Blue => "34",
            Self::Magenta => "35",
            Self::Cyan => "36",
            Self::White => "37",
            Self::Default => "39",
        }
    }
}

impl Display for Color {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_str("\x1b[")?;
        f.write_str(self.as_str())?;
        f.write_char('m')
    }
}

impl<S> Display for Colored<S>
        where S: Display {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        self.color.fmt(f)?;
        self.inner.fmt(f)?;
        f.write_str("\x1b[0m")
    }
}

pub trait Colorize: Sized {
    fn black(self) -> Colored<Self> {
        self.with_color(Color::Black)
    }

    fn red(self) -> Colored<Self> {
        self.with_color(Color::Red)
    }

    fn green(self) -> Colored<Self> {
        self.with_color(Color::Green)
    }

    fn yellow(self) -> Colored<Self> {
        self.with_color(Color::Yellow)
    }

    fn blue(self) -> Colored<Self> {
        self.with_color(Color::Blue)
    }

    fn magenta(self) -> Colored<Self> {
        self.with_color(Color::Magenta)
    }

    fn cyan(self) -> Colored<Self> {
        self.with_color(Color::Cyan)
    }

    fn white(self) -> Colored<Self> {
        self.with_color(Color::White)
    }

    fn with_color(self, color: Color) -> Colored<Self>;
}

impl<T> Colorize for T
        where T: Sized {
    fn with_color(self, color: Color) -> Colored<Self> {
        Colored {
            inner: self,
            color,
        }
    }
}

trait Stylized: Sized {
    fn stylized(self) -> Colored<Self>;
}

impl Stylized for log::Level {
    fn stylized(self) -> Colored<Self> {
        match self {
            Self::Error => self.red(),
            Self::Warn => self.yellow(),
            Self::Info => self.green(),
            Self::Debug => self.magenta(),
            Self::Trace => self.blue(),
        }
    }
}
