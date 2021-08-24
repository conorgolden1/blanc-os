use printer::{println, print};
use x86_64::{VirtAddr, structures::paging::{OffsetPageTable, PageTable, PhysFrame, Size4KiB, mapper::{MapToError}}};
use x86_64::structures::paging::FrameAllocator;

use crate::{PHYSICAL_MEMORY_OFFSET, active_level_4_table, phys::FRAME_ALLOCATOR};


/// Holds a physframe containing the page table to represent an
/// address space
pub struct AddressSpace {
    page_table_frame: PhysFrame,
}

impl AddressSpace {
    /// Allocate a new virtual address space
    pub fn new() -> Result<Self, MapToError<Size4KiB>> {
        let page_table_frame = unsafe {

            let frame = FRAME_ALLOCATOR
                .wait()
                .unwrap()
                .allocate_frame()
                .ok_or(MapToError::FrameAllocationFailed)?;

            let phys_addr = frame.start_address();
            let virt_addr =  VirtAddr::new(
                 phys_addr.as_u64() +
                 PHYSICAL_MEMORY_OFFSET.wait().unwrap().as_u64()
                );

              
            
            let new_page_table : *mut PageTable = virt_addr.as_mut_ptr();
            let new_page_table = &mut *new_page_table;

            let current_table = active_level_4_table();

            for i in 0..256 {
                new_page_table[i].set_unused();
            }
            
            for i in 256..512 {
                new_page_table[i] = current_table[i].clone();
            }
            
            frame
        };
        Ok(Self { page_table_frame})
    }

    /// Gives the active address space from CR3
    pub fn current_address_space() -> AddressSpace {
        let (page_table_frame, _) = x86_64::registers::control::Cr3::read();
        AddressSpace { page_table_frame }
    }

    /// Returns a mutable reference to the page table allocated for this
    /// address space.
    pub fn page_table(&mut self) -> &'static mut PageTable {
        unsafe {
            let phys_addr = self.page_table_frame.start_address();
            let virt_addr =  VirtAddr::new(
                phys_addr.as_u64() +
                PHYSICAL_MEMORY_OFFSET.wait().unwrap().as_u64()
               );

            let page_table_ptr: *mut PageTable = virt_addr.as_mut_ptr();

            &mut *page_table_ptr
        }
    }

    /// Wrap a offset page table around the page table referencing this task
    pub fn offset_page_table(&mut self) -> OffsetPageTable {
        unsafe { OffsetPageTable::new(self.page_table(), *PHYSICAL_MEMORY_OFFSET.wait().unwrap()) }
    }

    /// NOT WORKING Switch to this address space in the CR3 register
    ///
    /// # Safety
    /// TODO
    pub unsafe fn switch(&self) {

        println!("hello switch");
        x86_64::registers::control::Cr3::write(
            self.page_table_frame,
             x86_64::registers::control::Cr3::read().1
            );

        println!("goodbye switch");
        assert_eq!(x86_64::registers::control::Cr3::read().0, self.page_table_frame);
    }

    

}
