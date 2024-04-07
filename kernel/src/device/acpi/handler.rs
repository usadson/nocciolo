// Copyright (C) 2024 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use core::ptr::NonNull;
use acpi::{AcpiHandler, PhysicalMapping};
use x86_64::{PhysAddr, VirtAddr};
use x86_64::structures::paging::{Mapper, Page, PageTableFlags, PhysFrame, Size4KiB};
use crate::allocator::page::PageAllocator;
use crate::memory::{with_frame_allocator, with_mapper};
use crate::serial_println;

#[derive(Clone, Copy, Debug)]
pub(super) struct NoccioloAcpiHandler;

impl AcpiHandler for NoccioloAcpiHandler {
    unsafe fn map_physical_region<T>(&self, physical_address: usize, size: usize) -> PhysicalMapping<Self, T> {
        serial_println!("Mapping {physical_address:x} size {size:x}");
        let start = PhysAddr::new(physical_address as _).align_down(4096u64);
        let end = PhysAddr::new((physical_address + size) as _).align_up(4096u64);
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

        let page_count = (end - start) as usize / 4096;
        let virt = PageAllocator::allocate_n(page_count);

        do_map_region(start, end, virt, flags);

        let mapped_length = (end.as_u64() - start.as_u64()) as usize;

        let virtual_start = NonNull::<T>::new(virt.as_mut_ptr()).unwrap();

        let region = PhysicalMapping::new(physical_address, virtual_start, size, mapped_length, NoccioloAcpiHandler);
        serial_println!("Mapped {physical_address:x} {:p} {size:x} {mapped_length:x}", region.virtual_start().as_ptr());
        region
    }

    fn unmap_physical_region<T>(region: &PhysicalMapping<Self, T>) {
        serial_println!("Umapping {:x} {:p} {:x} {:x}", region.physical_start(), region.virtual_start().as_ptr(), region.region_length(), region.mapped_length());
        let ptr = region.virtual_start().as_ptr();
        let mut virt = VirtAddr::new(ptr as u64);

        let count = region.mapped_length() / 4096;
        for _ in 0..count {
            serial_println!("{:x} Is aligned: {}", virt.as_u64(), virt.is_aligned(4096u64));
            let page = Page::<Size4KiB>::from_start_address(virt).unwrap();
            with_mapper(|mapper| {
                mapper.unmap(page).expect("Failed to unmap ACPI");
            });
            virt += 4096;
        }
        serial_println!("Unmapped {:x} {:p} {:x} {:x}", region.physical_start(), region.virtual_start().as_ptr(), region.region_length(), region.mapped_length());
    }
}

fn do_map_region(start: PhysAddr, end: PhysAddr, virt_start: VirtAddr, flags: PageTableFlags) {
    let mut ptr = start;
    let mut virt = virt_start;
    while ptr < end {
        let page = Page::<Size4KiB>::from_start_address(virt).unwrap();

        with_mapper(|mapper| with_frame_allocator(|allocator| unsafe {
            // let frame = allocator.allocate_frame_from_physical(ptr).expect("Failed to allocate from same phys");
            let frame = PhysFrame::<Size4KiB>::containing_address(ptr);
            serial_println!("Did map {page:?}      {frame:?}");
            mapper.map_to(page, frame, flags, allocator).expect("Failed to map").flush();
        }));


        ptr += 4096;
        virt += 4096;
    }
}
