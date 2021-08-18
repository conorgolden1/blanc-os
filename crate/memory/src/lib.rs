//! OS page table mapping from physical to virtual
#![no_std]

use core::iter::{FlatMap, Map};

use bootloader::boot_info::{MemoryRegion, Optional};
use x86_64::{VirtAddr, structures::paging::{Mapper, Page, PageTable, PageTableFlags, RecursivePageTable, mapper::MapToError}};

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



use bootloader::boot_info::{MemoryRegions, MemoryRegionKind};
use x86_64::PhysAddr;

// /// A wrapper struct for the bootinfo memory map to allocate usable
// /// frames from the memory map
pub struct PhysFrameAllocator {
    usable_memory_region : MemoryRegion,
    bit_map_region : MemoryRegion,
}




impl PhysFrameAllocator {
    /// Initialize physical frame memory map 
    pub unsafe fn init(memory_regions: &'static MemoryRegions, page_table : &mut RecursivePageTable) -> Self {
        let mut usable_memory_region = MemoryRegion::empty();
        let mut bit_map_region = MemoryRegion::empty();

        let mut num_bit_map_frames : u64 = 0;

        usable_memory_region.kind = MemoryRegionKind::Usable;

        for memory_region in memory_regions.iter() {

            if memory_region.kind == MemoryRegionKind::Usable {
                num_bit_map_frames = ((memory_region.end - memory_region.start) / 4096 / 4096) + 1;

                bit_map_region.start = memory_region.start;
                bit_map_region.end = memory_region.start + (num_bit_map_frames * 4096) - 1;

                usable_memory_region.start = memory_region.start + (num_bit_map_frames * 4096);
                usable_memory_region.end = memory_region.end;
            }
            
        }


        map_bit_frames(bit_map_region, page_table, num_bit_map_frames).unwrap();
        
        PhysFrameAllocator {
            usable_memory_region,
            bit_map_region,
        }
    }
    // ///Navigate all of the frames in the table for usable frames and return an iterator of all
    // ///those frames
    fn get_frame(&self, index : u64) -> PhysFrame {
        let frame_addr = PhysAddr::new(self.usable_memory_region.start + (index * 4096));
        PhysFrame::<Size4KiB>::containing_address(frame_addr) 
    }
}

// TODO: CONSIDER TURNING BITMAP HANDLING ROUTINES INTO A STRUCT




use x86_64::structures::paging::{FrameAllocator, FrameDeallocator, Size4KiB, PhysFrame};

unsafe impl FrameAllocator<Size4KiB> for PhysFrameAllocator {
    ///Allocate the next available frame, increment the next counter and return that frame
    // TODO: CLEAN UP AND DOC
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let mut x = self.bit_map_region.start as *mut u64;
        while x < self.bit_map_region.end as *mut u64{
            let mut y = unsafe {* x } as u64;
            let clone = y.clone();
            let mut index = 0;
            
            while index < 64 {
                if y & 1 == 0 {
                    // Usable PhysFrame
                    unsafe {* x = clone | (0x01 << index)};
                    return Some(self.get_frame(x as u64 - self.bit_map_region.start + index))
                }
                index += 1;
                y = y >> 1;
            }

            unsafe { x = x.add(64) };
        }
        None
    }

    
    
}

use printer::{print, println};

impl FrameDeallocator<Size4KiB> for PhysFrameAllocator {
    /// Deallocate a frame in no longer in use
    ///
    /// This is done by clearing the bit in the bit_map to indicate that this
    /// frame is no longer in use
    ///
    /// # Safety
    /// The user must validate that the frame is no longer in use before
    /// deallocation
    // TODO: DOC AND CLEAN
    unsafe fn deallocate_frame(&mut self, frame: PhysFrame<Size4KiB>) {
        let start = frame.start_address().as_u64();
        let usable_addr = start - self.usable_memory_region.start ;
        let index  = usable_addr / 4096;
        
        let u64_byte = index / 64;
        let index = index % 64;

        let u64_byte_ptr = (self.bit_map_region.start + u64_byte) as *mut u64;
        *u64_byte_ptr = *u64_byte_ptr.clone() & !(1 << index);


    }
}





// TODO: DOCUMENT THIS!
fn map_bit_frames(bit_map_region: MemoryRegion, page_table: &mut RecursivePageTable, num_bit_map_frames : u64) ->  Result<(), MapToError<Size4KiB>> {
    let page_range = {
        let bit_map_start = VirtAddr::new(bit_map_region.start);
        let bit_map_end = VirtAddr::new(bit_map_region.end);
        let bit_map_start_page = Page::<Size4KiB>::containing_address(bit_map_start);
        let bit_map_end_page = Page::<Size4KiB>::containing_address(bit_map_end);
        Page::range_inclusive(bit_map_start_page, bit_map_end_page)
    };
    
    let bm_range = bit_map_region.start..bit_map_region.end;
    let frame_addresses = bm_range.step_by(4096);
    let mut frames = frame_addresses.map(|addr| PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(addr)));
    let mut empty_allocator = EmptyFrameAllocator;
   
    assert_eq!(page_range.count() as u64, num_bit_map_frames);

    for page in page_range {
        let frame = frames.next().ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe {
            page_table.map_to(page, frame, flags, &mut empty_allocator ).unwrap().flush()
        };
        assert_eq!(page_table.translate_page(page).unwrap(), frame);
    }
    Ok(())
    
}

pub struct EmptyFrameAllocator;

unsafe impl FrameAllocator<Size4KiB> for EmptyFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        None
    }
}

