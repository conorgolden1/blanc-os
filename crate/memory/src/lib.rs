//! OS Memory functionality and Structures
#![no_std]
#![feature(asm)]
#![feature(const_mut_refs,
    lang_items,
    alloc_error_handler)]


use bootloader::boot_info::Optional;
use core::{ops::Index};
use spin::{Mutex, Once};
use virt::deallocate_pages;
use x86_64::{registers::control::Cr3, structures::paging::{PageTable, RecursivePageTable}};

extern crate alloc;

pub mod allocator;
pub mod kpbox;
pub mod phys;
pub mod virt;

pub static RECURSIVE_INDEX: Once<Mutex<u16>> = Once::new();

pub static KERNEL_PAGE_TABLE: Once<Mutex<RecursivePageTable>> = Once::new();

///Using the recursive index find the level 4 table address and
///create a page table
///
/// # Safety
/// This function is unsafe because if the recursive index is not a valid index this
/// can result in undefined behavior
pub unsafe fn init(recursive_index: Optional<u16>) {
    RECURSIVE_INDEX.call_once(|| Mutex::new(recursive_index.into_option().unwrap()));
    let level_4_table = active_level_4_table();
    //mark_pages_unused();
    let kernel_page_table = RecursivePageTable::new(level_4_table).unwrap();
    KERNEL_PAGE_TABLE.call_once(|| Mutex::new(kernel_page_table));
}

///Find the base address of the active level page table with the recursive index
pub fn active_level_4_table() -> &'static mut PageTable {
    let r = *RECURSIVE_INDEX.wait().unwrap().lock() as u64;
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
pub fn swap_to_kernel_table() {
    *RECURSIVE_INDEX.wait().unwrap().lock() = 508;
    unsafe {Cr3::write( KERNEL_PAGE_TABLE.wait().unwrap().lock().level_4_table().index(508).frame().unwrap(), Cr3::read().1)};
    x86_64::instructions::tlb::flush_all();

}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}