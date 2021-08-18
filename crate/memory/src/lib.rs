//! OS page table mapping from physical to virtual
#![no_std]

use spin::Once;
use x86_64::{PhysAddr, VirtAddr, structures::paging::{PageTable, OffsetPageTable, 
            PhysFrame, Size4KiB, FrameAllocator}};


pub static PHYS_MEM_OFFSET: Once<VirtAddr> = Once::new();

///Get the memory address of the level 4 Page table and map it in virtual memory,
///Then return the new offset page table with the level 4 mapping
///The physical memory offset is the offset to the bootinfo physical memory frame
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    PHYS_MEM_OFFSET.call_once(|| { physical_memory_offset.clone() });
    let level_4_table = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}


///Map the active level 4 table to a virtual page table in memory
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    // Get the start address of the lvl 4 table frame
    let phys = level_4_table_frame.start_address();
    // Create a virtual address mapped the to physical address
    let virt = physical_memory_offset + phys.as_u64();
    // Create a page table obj at the virtual address that is subsequently mapped to
    // the physical frame
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    &mut *page_table_ptr

}



// use bootloader::bootinfo::{MemoryMap, MemoryRegionType};


// /// A wrapper struct for the bootinfo memory map to allocate usable
// /// frames from the memory map
// pub struct BootInfoFrameAllocator {
//     memory_map: &'static MemoryMap,
//     next: usize,
// }

// impl BootInfoFrameAllocator {
//     /// Initialize the memory map 
//     pub unsafe fn init(memory_map: &'static MemoryMap) -> Self {
//         BootInfoFrameAllocator {
//             memory_map,
//             next: 0,
//         }
//     }
//     ///Navigate all of the frames in the table for usable frames and return an iterator of all
//     ///those frames
//     fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
//         let regions = self.memory_map.iter();
//         let usable_regions = regions.filter(|r| r.region_type == MemoryRegionType::Usable);

//         let addr_ranges = usable_regions.map(|r| r.range.start_addr()..r.range.end_addr());

//         let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));

//         frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))

//     }
// }

// unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
//     ///Allocate the next available frame, increment the next counter and return that frame
//     fn allocate_frame(&mut self) -> Option<PhysFrame> {
//         let frame = self.usable_frames().nth(self.next);
//         self.next += 1;
//         frame
//     }
// }

