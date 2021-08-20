use x86_64::structures::paging::{PhysFrame, Size4KiB, mapper::MapToError};


/// Holds a physframe containing the page table to represent an
/// address space
pub struct AddressSpace {
    page_table_frame: PhysFrame,
}

impl AddressSpace {
    ///// Allocate a new virtual address space
    // pub fn new() -> Result<Self, MapToError<Size4KiB>> {
    //     let page_table_frame = {
    //         let frame = F
    //     }
    // }
}