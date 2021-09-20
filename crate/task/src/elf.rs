//! Allocates and loads an given ELF buffer into memory at
//! an offset 
// 
// Considerations
// 
// We might want to consider keeping track of each segment loaded into memory and there type
// given there size and there starting address, so when a page fault occurs we can reference
// the tasks segment size to see if it is out of bounds or more memory can be allocated to it.
// Also the way that this is set up, is meant for processes that are position independent executables
use elfloader::{ElfLoader, TypeRela64};
use memory::{active_level_4_table, phys::FRAME_ALLOCATOR};

use x86_64::{
    align_up,
    structures::paging::{
        FrameAllocator, Mapper, Page, PageSize, PageTableFlags, RecursivePageTable, Size4KiB,
    },
    VirtAddr,
};
extern crate alloc;
use alloc::vec::Vec;

pub fn align_bin(bin: &[u8]) -> Vec<u8> {
    let mut vec = Vec::<u8>::new();
    vec.resize(bin.len(), 0);
    vec.clone_from_slice(bin);
    vec
}


/// This struct represents a loaded ELF executable in memory starting at an
/// offset. Using the [elfloader] crate we allocate memory for the elf
/// and load the elf into the new memory section at a given offset
pub struct ElfMemory {
    /// Virtual base where the elf mapping starts at
    vbase: u64,
}

impl ElfMemory {
    /// Create a new ElfMemory at an offset in virtual memory
    pub fn new(vbase: u64) -> Self {
        Self { vbase }
    }

    /// Get a reference to loaded elf base memory address.
    pub fn vbase(&self) -> &u64 {
        &self.vbase
    }
}

impl ElfLoader for ElfMemory {
    /// Allocate the required memory for the elf from the offset
    /// for each of the loadable elf header
    fn allocate(
        &mut self,
        load_headers: elfloader::LoadableHeaders,
    ) -> Result<(), elfloader::ElfLoaderErr> {
        let mut current_pt = RecursivePageTable::new(active_level_4_table()).unwrap();
        for header in load_headers {
            let _flags = header.flags();
            let ptf = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

            let start = self.vbase + header.virtual_addr();
            let end = align_up(start + header.mem_size(), Size4KiB::SIZE);
            let start_virt = VirtAddr::new(start);
            let end_virt = VirtAddr::new(end - 1);
            let start_page = Page::<Size4KiB>::containing_address(start_virt);
            let end_page = Page::<Size4KiB>::containing_address(end_virt);
            let page_range = Page::range_inclusive(start_page, end_page);
            for page in page_range {
                let frame = FRAME_ALLOCATOR.wait().unwrap().allocate_frame().unwrap();

                unsafe {
                    let result = current_pt.map_to_with_table_flags(
                        page,
                        frame,
                        ptf,
                        PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
                        FRAME_ALLOCATOR.wait().as_mut().unwrap(),
                    );

                    match result {
                        Ok(mapper_flush) => mapper_flush.flush(),
                        Err(err) => panic!("{:#?} => {:#?}", page, err),
                    }
                }
            }
        }

        Ok(())
    }

    /// Load the elf segment from the static memory location
    /// into the buffer obtained from allocate
    fn load(
        &mut self,
        flags: elfloader::Flags,
        base: elfloader::VAddr,
        region: &[u8],
    ) -> Result<(), elfloader::ElfLoaderErr> {
        let start = self.vbase + base;

        let start_ptr = start as *mut u8;

        for (offset, entry) in region.iter().enumerate() {
            unsafe {
                *(start_ptr.add(offset)) = *entry;
            }
        }

        Ok(())
    }

    /// TODO
    fn relocate(
        &mut self,
        entry: &elfloader::Rela<elfloader::P64>,
    ) -> Result<(), elfloader::ElfLoaderErr> {
        let typ = TypeRela64::from(entry.get_type());
        let addr: *mut u64 = (self.vbase + entry.get_offset()) as *mut u64;

        match typ {
            TypeRela64::R_RELATIVE => {
                unsafe { *addr = self.vbase() + entry.get_addend() };
                Ok(())
            }
            _ => todo!("{:#?} else not yet implemented", typ)
        }
        
    }
}


