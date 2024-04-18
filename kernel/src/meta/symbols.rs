// Copyright (C) 2024 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use core::{arch::asm, ptr::slice_from_raw_parts};

use bootloader_api::BootInfo;
use conquer_once::spin::OnceCell;
use elf::{endian::NativeEndian, ElfBytes};
use lazy_static::lazy_static;
use log::warn;

lazy_static! {
    static ref ELF: OnceCell<Option<ElfBytes<'static, NativeEndian>>> = OnceCell::uninit();
}

pub(super) fn init(boot_info: &'static BootInfo) {
    let data = match ElfBytes::<NativeEndian>::minimal_parse(get_elf_slice(boot_info)) {
        Ok(data) => data,
        Err(e) => {
            warn!("Failed to parse ELF: {e}");
            return;
        }
    };

    ELF.init_once(|| Some(data));
}

pub struct Backtrace;

impl Backtrace {
    pub fn capture() -> BacktraceIterator {
        BacktraceIterator {
            ptr: Self::read_rbp() as *const u64,
        }
    }

    pub fn capture_from(ptr: *const u64) -> BacktraceIterator {
        BacktraceIterator {
            ptr,
        }
    }

    #[inline(always)]
    fn read_rbp() -> u64 {
        let rbp: u64;
        unsafe {
            asm!("mov %rbp, {0}", out(reg) rbp, options(att_syntax));
        }
        rbp
    }
}

pub struct BacktraceIterator {
    ptr: *const u64,
}

impl Iterator for BacktraceIterator {
    type Item = BacktraceFrame;

    fn next(&mut self) -> Option<Self::Item> {
        if self.ptr.is_null() {
            return None;
        }

        let ptr = self.ptr;
        self.ptr = unsafe { *ptr } as *const u64;
        Some(BacktraceFrame {
            ptr,
        })
    }
}

pub struct BacktraceFrame {
    ptr: *const u64,
}

impl BacktraceFrame {
    pub fn pointer(&self) -> *const u64 {
        self.ptr
    }

    pub fn symbol(&self) -> Option<&'static str> {
        resolve(self.ptr as _)
    }
}

pub fn resolve(offset: u64) -> Option<&'static str> {
    let elf = ELF.get()?.as_ref()?;

    let (sym_tab, str_tab) = elf.symbol_table().ok()??;

    for sym in sym_tab {
        if sym.st_value < offset {
            continue;
        }

        if sym.st_value + sym.st_size > offset {
            continue;
        }

        if sym.st_name == 0 {
            return None;
        }

        return str_tab.get(sym.st_name as usize).ok();
    }

    None
}

fn get_elf_slice(boot_info: &'static BootInfo) -> &'static [u8] {
    let data = boot_info.kernel_image_offset as *const u8;
    let len = boot_info.kernel_len as usize;

    unsafe {
        &*slice_from_raw_parts(data, len)
    }
}
