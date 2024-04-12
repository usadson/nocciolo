use core::{default, fmt, ptr::slice_from_raw_parts_mut};
use bootloader_api::info::{FrameBuffer, FrameBufferInfo, PixelFormat};
use lazy_static::lazy_static;
use noto_sans_mono_bitmap::{FontWeight, RasterHeight, RasterizedChar};

use crate::serial_println;

static EMPTY: &[u8] = &[];

lazy_static! {
    pub static ref WRITER: spin::Mutex<Writer> = spin::Mutex::new(Writer {
        info: FrameBufferInfo {
            byte_len: 0,
            width: 0,
            height: 0,
            pixel_format: PixelFormat::U8,
            bytes_per_pixel: 0,
            stride: 0,
        },
        x_pos: 0,
        y_pos: 0,
        last_width: 0,
        framebuffer: unsafe { &mut *slice_from_raw_parts_mut(EMPTY.as_ptr() as *mut _, 0) },
        color: Color::White,
        state: Default::default(),
    });
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

impl Color {
    pub fn rgb(&self) -> [u8; 4] {
        let alpha = 0x00;
        match self {
            Self::Black => [0x00, 0x00, 0x00, alpha],
            Self::Blue => [0x00, 0x00, 0xFF, alpha],
            Self::Green => [0x00, 0xFF, 0x00, alpha],
            Self::Cyan => [0x00, 0xFF, 0xFF, alpha],
            Self::Red => [0xFF, 0x00, 0x00, alpha],
            Self::Magenta => [0xFF, 0x00, 0xFF, alpha],
            Self::Brown => [0xFF, 0xFF, 0x80, alpha], // check
            Self::LightGray => [0x55, 0x55, 0x55, alpha],
            Self::DarkGray => [0xAA, 0xAA, 0xAA, alpha],
            Self::LightBlue => [0x00, 0x00, 0x80, alpha],
            Self::LightGreen => [0x00, 0x80, 0x80, alpha],
            Self::LightCyan => [0x00, 0x80, 0x80, alpha],
            Self::LightRed => [0x80, 0x00, 0x00, alpha],
            Self::Pink => [0xFF, 0xCC, 0xCC, alpha],
            Self::Yellow => [0xFF, 0xFF, 0x00, alpha],
            Self::White => [0xFF, 0xFF, 0xFF, alpha],
        }
    }
}

use noto_sans_mono_bitmap::get_raster_width;


fn get_char_raster(c: char) -> RasterizedChar {
    fn get(c: char) -> Option<RasterizedChar> {
        noto_sans_mono_bitmap::get_raster(
            c,
            font_constants::FONT_WEIGHT,
            font_constants::CHAR_RASTER_HEIGHT,
        )
    }
    get(c).unwrap_or_else(|| get(font_constants::BACKUP_CHAR).expect("Should get raster of backup char."))
}

mod font_constants {
    use super::*;

    pub const LINE_SPACING: usize = 2;
    pub const LETTER_SPACING: usize = 0;
    pub const BORDER_PADDING: usize = 1;

    pub const CHAR_RASTER_HEIGHT: RasterHeight = RasterHeight::Size16;
    pub const CHAR_RASTER_WIDTH: usize = get_raster_width(FontWeight::Regular, CHAR_RASTER_HEIGHT);
    pub const BACKUP_CHAR: char = '�';
    pub const FONT_WEIGHT: FontWeight = FontWeight::Regular;
}

pub struct Writer {
    framebuffer: &'static mut [u8],
    info: FrameBufferInfo,
    last_width: usize,
    x_pos: usize,
    y_pos: usize,
    color: Color,
    state: WriterState,
}

#[derive(Default, Clone, Copy)]
enum WriterState {
    #[default]
    Normal,
    Escape,
    FirstCode,
    SecondCode(char),
    Finishing(char, char),
    Color(Color),
}

impl WriterState {
    pub fn feed(&mut self, ch: char) -> bool {
        match self {
            Self::Normal => {
                if ch != '\x1b' {
                    *self = Self::Normal;
                    return true;
                }

                *self = Self::Escape;
                false
            }

            Self::Escape => {
                if ch != '[' {
                    *self = Self::Normal;
                    return true;
                }

                *self = Self::FirstCode;
                false
            }

            Self::FirstCode => {
                if ch == '0' {
                    *self = Self::Finishing(ch, ch);
                } else {
                    *self = Self::SecondCode(ch);
                }
                false
            }

            Self::SecondCode(first) => {
                *self = Self::Finishing(*first, ch);
                false
            }

            Self::Finishing(first, second) => {
                if ch != 'm' {
                    *self = Self::Normal;
                    return true;
                }

                if *first == '0' {
                    *self = Self::Color(Color::White);
                    return false;
                }

                if *first != '3' {
                    *self = Self::Normal;
                    return false;
                }

                *self = Self::Color(match *second {
                    '0' => Color::Black,
                    '1' => Color::Red,
                    '2' => Color::Green,
                    '3' => Color::Yellow,
                    '4' => Color::Blue,
                    '5' => Color::Magenta,
                    '6' => Color::Cyan,
                    '7' => Color::White,
                    _ => {
                        *self = Self::Normal;
                        return false;
                    }
                });

                false
            }

            Self::Color(..) => true,
        }
    }
}

impl Writer {
    pub fn set_buffer(&mut self, buf: &'static [u8]) {
        let data = buf.as_ptr() as *mut u8;
        let len = buf.len();
        let slice = unsafe { &mut *slice_from_raw_parts_mut(data, len) };
        self.framebuffer = slice;

        self.clear();
    }

    pub fn set_fb(&mut self, fb: &'static FrameBuffer) {
        self.set_buffer(fb.buffer());
        self.info = fb.info();

        serial_println!("FB: {:#?}", self.info);
    }

    fn newline(&mut self) {
        self.y_pos += font_constants::CHAR_RASTER_HEIGHT.val() + font_constants::LINE_SPACING;
        self.carriage_return()
    }

    fn carriage_return(&mut self) {
        self.x_pos = font_constants::BORDER_PADDING;
    }

    pub fn clear(&mut self) {
        self.x_pos = font_constants::BORDER_PADDING;
        self.y_pos = font_constants::BORDER_PADDING;
        self.framebuffer.fill(0);
    }

    fn width(&self) -> usize {
        self.info.width
    }

    fn height(&self) -> usize {
        self.info.height
    }

    fn write_char(&mut self, c: char) {
        if !self.state.feed(c) {

            if let WriterState::Color(color) = self.state {
                self.state = WriterState::Normal;
                self.color = color;
            }

            return;
        }

        match c {
            '\n' => self.newline(),
            '\r' => self.carriage_return(),
            c => {
                let new_xpos = self.x_pos + font_constants::CHAR_RASTER_WIDTH;
                if new_xpos >= self.width() {
                    self.newline();
                }
                let new_ypos =
                    self.y_pos + font_constants::CHAR_RASTER_HEIGHT.val() + font_constants::BORDER_PADDING;
                if new_ypos >= self.height() {
                    self.clear();
                }
                self.write_rendered_char(get_char_raster(c));
            }
        }
    }

    fn write_rendered_char(&mut self, rendered_char: RasterizedChar) {
        for (y, row) in rendered_char.raster().iter().enumerate() {
            for (x, byte) in row.iter().enumerate() {
                self.write_pixel(self.x_pos + x, self.y_pos + y, *byte);
            }
        }
        self.last_width = rendered_char.width();
        self.x_pos += self.last_width + font_constants::LETTER_SPACING;
    }

    pub fn backspace(&mut self) {
        self.x_pos -= self.last_width;
        self.write_char(' ');
        self.x_pos -= self.last_width;
    }

    fn write_pixel(&mut self, x: usize, y: usize, intensity: u8) {
        let pixel_offset = y * self.info.stride + x;
        let color = self.get_color(intensity);
        let bytes_per_pixel = self.info.bytes_per_pixel;
        let byte_offset = pixel_offset * bytes_per_pixel;
        self.framebuffer[byte_offset..(byte_offset + bytes_per_pixel)]
            .copy_from_slice(&color[..bytes_per_pixel]);
        let _ = unsafe { core::ptr::read_volatile(&self.framebuffer[byte_offset]) };
    }

    fn write_string(&mut self, s: &str) {
        for c in s.chars() {
            self.write_char(c);
        }
    }

    fn get_color(&mut self, intensity: u8) -> [u8; 4] {
        let mut color = self.color.rgb();

        let intensity = intensity as usize;
        for x in color.iter_mut() {
            let value = *x as usize;
            *x = ((value * intensity) / 255) as u8;
        }

        match self.info.pixel_format {
            PixelFormat::Rgb => color,
            PixelFormat::Bgr => [color[2], color[1], color[0], color[3]],
            PixelFormat::U8 => [if intensity > 200 { 0xf } else { 0 }, 0, 0, 0],
            other => {
                self.info.pixel_format = PixelFormat::Rgb;
                panic!("pixel format {:?} not supported in logger", other)
            }
        }
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_text_buffer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    interrupts::without_interrupts(|| {
        WRITER.lock().write_fmt(args).unwrap();
    });
}
