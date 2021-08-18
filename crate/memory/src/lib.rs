//! OS page table mapping from physical to virtual
#![no_std]

use bootloader::boot_info::Optional;
use x86_64::{structures::paging::{PageTable, RecursivePageTable}};


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


///Map the active level 4 table to a virtual page table in memory
unsafe fn active_level_4_table(recursive_index: u16) -> &'static mut PageTable {
    let r = recursive_index as u64;
    let sign: u64;

    if r > 255 {
        sign = 0o177777 << 48;
    } else {
        sign = 0;
    }

    let level_4_table_addr =
    sign | (r << 39) | (r << 30) | (r << 21) | (r << 12);
    let level_4_table = level_4_table_addr as *mut PageTable;
    &mut *level_4_table
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

