//! OS Memory functionality and Structures
#![no_std]
#![feature(asm)]

use bootloader::boot_info::{Optional};
use spin::Once;
use x86_64::{VirtAddr, structures::paging::{OffsetPageTable, PageTable}};


pub mod virt;
pub mod phys;

pub static PHYSICAL_MEMORY_OFFSET : Once<VirtAddr> = Once::new();

///Using the recursive index find the level 4 table address and
///create a page table
///
/// # Safety
/// This function is unsafe because if the recursive index is not a valid index this
/// can result in undefined behavior
pub unsafe fn init(physical_memory_offset: Optional<u64>) -> OffsetPageTable<'static> {
    PHYSICAL_MEMORY_OFFSET.call_once(|| VirtAddr::new(physical_memory_offset.into_option().unwrap()));
    let level_4_table = active_level_4_table();
    OffsetPageTable::new(level_4_table, *PHYSICAL_MEMORY_OFFSET.wait().unwrap())
}


///Find the base address of the active level page table with the recursive index
unsafe fn active_level_4_table() -> &'static mut PageTable {

    let (phys_addr, _) = x86_64::registers::control::Cr3::read();
    let virt_addr = VirtAddr::new(
        PHYSICAL_MEMORY_OFFSET.wait().unwrap().as_u64() + phys_addr.start_address().as_u64());

    let page_table_ptr: *mut PageTable = virt_addr.as_mut_ptr();

    &mut *page_table_ptr
}

