//! Allocates and loads an given ELF buffer into memory at
//! an offset 
// 
// Considerations
// 
// We might want to consider keeping track of each segment loaded into memory and there type
// given there size and there starting address, so when a page fault occurs we can reference
// the tasks segment size to see if it is out of bounds or more memory can be allocated to it.
// Also the way that this is set up, is meant for processes that are position independent executables
use elfloader::{ElfBinary, ElfLoader};
use memory::{active_level_4_table, phys::FRAME_ALLOCATOR};

use x86_64::{
    align_up,
    structures::paging::{
        FrameAllocator, Mapper, Page, PageSize, PageTableFlags, RecursivePageTable, Size4KiB,
    },
    VirtAddr,
};

/// Allocate and load the binary from the current active page table
/// this function sets the new elf binary at a memory offset of 0x1000_0000
pub fn load_elf(bin: &[u8]) -> ElfBinary {
    let elf = ElfBinary::new(bin).unwrap();
    let mut loader = ElfMemory::new(0x1000_0000);
    elf.load(&mut loader).unwrap();
    elf
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
            let end_virt = VirtAddr::new(end);
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

                    result.unwrap().flush();
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
        todo!()
    }
}
