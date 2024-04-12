// Copyright (C) 2024 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use crate::vga_text_buffer::{self, WRITER};

pub struct Console;

impl Console {
    pub fn backspace() {
        let mut writer = WRITER.lock();
        writer.backspace();
    }
}
