//! This module provides structures and functionality for handling the virtual memory 
//! allocations and management for tasks

use alloc::collections::LinkedList;
use x86_64::{VirtAddr, structures::{
    idt::PageFaultErrorCode, 
    paging::{
        FrameAllocator, 
        Mapper, 
        OffsetPageTable, 
        Page, 
        PageSize, 
        PageTableFlags, 
        Size4KiB}}};

use crate::phys::FRAME_ALLOCATOR;

use self::address_space::AddressSpace;

extern crate alloc;

pub mod address_space;

/// A linked list of all of the memory areas contained within a task
/// blanc-os currently supports ELF file executables and a task's individual
/// memory areas are contained in this list
///
/// See [ELF](https://wiki.osdev.org/ELF)
pub struct TaskMemoryMap {
    mappings : LinkedList<TaskMapping>
}

impl TaskMemoryMap {
    /// Create a new TaskMemoryMap, is initialized with a empty mappings list
    /// TODO : consider initializing the mappings from ELF from here
    fn new() -> TaskMemoryMap {
        TaskMemoryMap {
            mappings : LinkedList::new(),
        }
    }

    /// The memory maps page fault handler discovers whether to expand memory to the task
    /// or to deny it based on the [TaskMapping] protocol
    fn page_fault_handler(&mut self, fault_err_code : PageFaultErrorCode, address : VirtAddr) -> bool {
        if let Some(map) = self.mappings.
            iter_mut().
            find(|e| address >= e.start_addr && address < e.end_addr) 
        {
            // Task memory area is uninitalized
            if map.protocol.is_empty() {
                return false
            }

            // Task memory area tried to write to and does not have write access
            if fault_err_code.contains(PageFaultErrorCode::CAUSED_BY_WRITE) && !map.protocol.contains(MMapProt::PROT_WRITE) {
                return false
            }

            // Task memory area tried to execute but does not have execution status
            if fault_err_code.contains(PageFaultErrorCode::INSTRUCTION_FETCH) && !map.protocol.contains(MMapProt::PROT_EXEC) {
                return false
            }

            let is_private = map.flags.contains(MMapFlags::MAP_PRIVATE);
            let is_anon = map.flags.contains(MMapFlags::MAP_ANONYOMUS);

            let mut address_space = AddressSpace::current_address_space();
            let mut page_table = address_space.offset_page_table();

            let result: bool = match (is_private, is_anon) {
                // Task is private and is anonymous
                (true, true) => {
                    map.page_fault_private_anon(&mut page_table, fault_err_code, address)
                }
                // Task is private and not anonymous
                (true, false) => {
                    map.page_fault_private_file(&mut page_table, fault_err_code, address)
                }
                // Task not private and is anonymous
                (false, true) => unreachable!(),

                // Task not private and not anonymous
                (false, false) => unimplemented!()
            };
            result
        } else {
            // Else the mapping does not exist, so return false.
            false
        }
    }

}


/// A individual mapping in a Task's address space.
/// Tasks contain isolated memory area's such as the task stack,
/// data/heap and text/code sections. These individual areas are represented in a task
/// mapping. 
///
/// See [virtual address space](https://en.wikipedia.org/wiki/Virtual_address_space#/media/File:Virtual_address_space_and_physical_address_space_relationship.svg)
#[derive(Clone)]
pub struct TaskMapping {
    protocol: MMapProt,
    flags: MMapFlags,

    start_addr: VirtAddr,
    end_addr: VirtAddr,

    file: Option<MMapFile>,
}



impl TaskMapping {
    /// If the page fault does not conatin a protection violation then this function will expand
    /// the task's memory a single page
    pub(crate) fn page_fault_private_anon(&self, page_table: &mut OffsetPageTable, fault_err_code: PageFaultErrorCode, address: VirtAddr) -> bool {
        let addr_aligned = address.align_down(Size4KiB::SIZE);
        if !fault_err_code.contains(PageFaultErrorCode::PROTECTION_VIOLATION) { 
            let frame = unsafe {
                FRAME_ALLOCATOR.wait().unwrap().allocate_frame().expect("Failed to allocate frame")
            };
            unsafe {
                page_table.map_to(
                    Page::containing_address(addr_aligned),
                    frame,
                    PageTableFlags::USER_ACCESSIBLE | PageTableFlags::PRESENT | self.protocol.into(),
                    &mut FRAME_ALLOCATOR.wait().unwrap()
                ).unwrap().flush();
            }
            return true
        } 
        false
    }
    
    /// TODO
    pub(crate) fn page_fault_private_file(&self, page_table: &mut OffsetPageTable, fault_err_code: PageFaultErrorCode, address: VirtAddr) -> bool {
        todo!()
    }

}


/// Virtual Address Area where a file loaded into memory for caching purposes is
#[derive(Clone)]
pub struct MMapFile {
    offset: usize,
    file: &'static [u8],
    size: usize,
}

bitflags::bitflags! {

    /// Memory map protection arguements
    /// 
    /// See [Mmap Prot](https://man7.org/linux/man-pages/man2/mmap.2.html)
    pub struct MMapProt: usize {
        ///Pages may not be accessed.
        const PROT_NONE = 0x0;

        ///Pages may be read.
        const PROT_READ = 0x1;

        ///Pages may be written.
        const PROT_WRITE = 0x2;

        ///Pages may be executed.
        const PROT_EXEC = 0x4;
    }


    /// Memory map flags
    ///
    /// The flags argument determines whether updates to the mapping are
    /// visible to other processes mapping the same region, and whether
    /// updates are carried through to the underlying file.
    ///
    /// See [flags argument](https://man7.org/linux/man-pages/man2/mmap.2.html)
    pub struct MMapFlags: usize {
        
        /// Create a private copy-on-write mapping.  Updates to the
        /// mapping are not visible to other processes mapping the
        /// same file, and are not carried through to the underlying
        /// file.  It is unspecified whether changes made to the file
        /// after the mmap() call are visible in the mapped region.
        const MAP_PRIVATE = 0x1;

        /// Share this mapping.  Updates to the mapping are visible to
        /// other processes mapping the same region, and (in the case
        /// of file-backed mappings) are carried through to the
        /// underlying file.
        const MAP_SHARED = 0x2;

        /// Don't interpret addr as a hint: place the mapping at
        /// exactly that address.  addr must be suitably aligned: for
        /// most architectures a multiple of the page size is
        /// sufficient
        const MAP_FIXED = 0x4;

        /// The mapping is not backed by any file; its contents are
        /// initialized to zero.
        const MAP_ANONYOMUS = 0x8;
    }
}



impl From<MMapProt> for PageTableFlags {
    /// Convert ELF protection flags into Page Flags
    fn from(e: MMapProt) -> Self {
        let mut res = PageTableFlags::empty();

        if !e.contains(MMapProt::PROT_EXEC) {
            res.insert(PageTableFlags::NO_EXECUTE);
        }

        if e.contains(MMapProt::PROT_WRITE) {
            res.insert(PageTableFlags::WRITABLE);
        }

        res
    }
}

