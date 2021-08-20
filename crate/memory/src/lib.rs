//! OS page table mapping from physical to virtual
#![no_std]
#![feature(once_cell)]

use core::iter::{FlatMap, Map};
use bootloader::boot_info::{MemoryRegion, Optional};
use x86_64::{VirtAddr, structures::paging::{Mapper, Page, PageTable, PageTableFlags, RecursivePageTable, mapper::MapToError}};

pub mod frame;
pub mod address_space;

///Using the recursive index find the level 4 table address and
///create a page table
///
/// # Safety
/// This function is unsafe because if the recursive index is not a valid index this
/// can result in undefined behavior
pub unsafe fn init(recursive_index: Optional<u16>) -> RecursivePageTable<'static> {
    let level_4_table = active_level_4_table(*recursive_index.as_ref().unwrap());
    RecursivePageTable::new(level_4_table).unwrap()
}


///Find the base address of the active level page table with the recursive index
unsafe fn active_level_4_table(recursive_index: u16) -> &'static mut PageTable {
    let r = recursive_index as u64;
    let sign: u64;

    if r > 255 {
        sign = 0o177777 << 48;
    } else {
        sign = 0;
    }

    let level_4_table_addr = sign | (r << 39) | (r << 30) | (r << 21) | (r << 12);
    let level_4_table = level_4_table_addr as *mut PageTable;

    &mut *level_4_table
}




pub struct PageAllocator {
    page_table : RecursivePageTable<'static>,
} 
