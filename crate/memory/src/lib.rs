//! OS Memory functionality and Structures
#![no_std]
#![feature(asm)]
#![feature(alloc_error_handler)]
#![feature(const_mut_refs)]
#![feature(lang_items)]
use accessor::single::ReadWrite;
use bootloader::boot_info::Optional;
use core::{
    convert::{TryFrom, TryInto},
    num::NonZeroUsize,
};
use os_units::Bytes;
use phys::FRAME_ALLOCATOR;
use spin::{Mutex, Once};
use virt::deallocate_pages;
use x86_64::{
    structures::paging::{
        page_table::PageTableEntry, FrameDeallocator, Mapper, OffsetPageTable, Page, PageSize,
        PageTable, PageTableFlags, RecursivePageTable, Size4KiB,
    },
    PhysAddr, VirtAddr,
};

extern crate alloc;

pub mod allocator;
pub mod kpbox;
pub mod phys;
pub mod virt;

pub static RECURSIVE_INDEX: Once<u16> = Once::new();

pub static KERNEL_PAGE_TABLE: Once<Mutex<RecursivePageTable>> = Once::new();

///Using the recursive index find the level 4 table address and
///create a page table
///
/// # Safety
/// This function is unsafe because if the recursive index is not a valid index this
/// can result in undefined behavior
pub unsafe fn init(recursive_index: Optional<u16>) {
    RECURSIVE_INDEX.call_once(|| recursive_index.into_option().unwrap());
    let level_4_table = active_level_4_table();
    //mark_pages_unused();
    let kernel_page_table = RecursivePageTable::new(level_4_table).unwrap();
    KERNEL_PAGE_TABLE.call_once(|| Mutex::new(kernel_page_table));
}

///Find the base address of the active level page table with the recursive index
pub fn active_level_4_table() -> &'static mut PageTable {
    let r = *RECURSIVE_INDEX.wait().unwrap() as u64;
    let sign: u64;

    if r > 255 {
        sign = 0o177777 << 48;
    } else {
        sign = 0;
    }

    let level_4_table_addr = sign | (r << 39) | (r << 30) | (r << 21) | (r << 12);
    let level_4_table = level_4_table_addr as *mut PageTable;

    unsafe { &mut *level_4_table }
}

///TODO
fn mark_pages_unused() {
    let page_table = active_level_4_table();

    for i in 4..510 {
        page_table[i].set_unused();
    }
}

// /// # Safety
// pub unsafe fn get_inner_table(pte : &PageTableEntry) -> &'static mut PageTable {
//     let table_frame = pte.frame().unwrap();
//     let virt_addr = VirtAddr::new(PHYSICAL_MEMORY_OFFSET.wait().unwrap().as_u64() + table_frame.start_address().as_u64());
//     let page_table_ptr: *mut PageTable = virt_addr.as_mut_ptr();
//     &mut *page_table_ptr
// }
