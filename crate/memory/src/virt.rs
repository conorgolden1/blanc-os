use crate::{
    active_level_4_table,
    phys::{BYTES_AVAILABLE_RAM, FRAME_ALLOCATOR},
};
use accessor::single::ReadWrite;
use core::{
    convert::{TryFrom, TryInto},
    num::NonZeroUsize,
};
use os_units::{self, Bytes, NumOfPages};
use x86_64::{
    structures::paging::{
        FrameAllocator, FrameDeallocator, Mapper, Page, PageSize, PageTableFlags,
        RecursivePageTable, Size4KiB, Translate,
    },
    PhysAddr, VirtAddr,
};

/// Search for any free address space that has a consistent number of pages in the
/// active level 4 page table
/// O(n)
fn search_free_addr(num_pages: NumOfPages<Size4KiB>) -> Option<VirtAddr> {
    let mut cnt = 0;
    let mut start = None;
    for addr in (0..(*BYTES_AVAILABLE_RAM.wait().unwrap()) as usize)
        .step_by(usize::try_from(Size4KiB::SIZE).unwrap())
    {
        let addr = VirtAddr::new(addr as _);
        if available(addr) {
            if start.is_none() {
                start = Some(addr);
            }

            cnt += 1;

            if cnt >= num_pages.as_usize() {
                return start;
            }
        } else {
            cnt = 0;
            start = None;
        }
    }

    None
}

/// Check to see if an address is free from the current active level page table
fn available(addr: VirtAddr) -> bool {
    let pml4 = RecursivePageTable::new(active_level_4_table()).unwrap();
    pml4.translate_addr(addr).is_none() && !addr.is_null()
}

/// Deallocate # of pages in a linear space starting from the Virtual Address
pub fn deallocate_pages(virt: VirtAddr, num_of_pages: NumOfPages<Size4KiB>) {
    let mut page_table = RecursivePageTable::new(active_level_4_table()).unwrap();
    for i in 0..u64::try_from(num_of_pages.as_usize()).unwrap() {
        let page = Page::<Size4KiB>::from_start_address(virt + Size4KiB::SIZE * i).unwrap();

        let (frame, flush) = page_table.unmap(page).unwrap();
        unsafe {
            FRAME_ALLOCATOR
                .wait()
                .unwrap()
                .inner
                .lock()
                .deallocate_frame(frame)
        };
        flush.flush();
    }
}

/// Allocate # of pages starting at a virtual address to random frames
pub fn allocate_pages(virt: VirtAddr, num_of_pages: NumOfPages<Size4KiB>) {
    let mut page_table = RecursivePageTable::new(active_level_4_table()).unwrap();

    for i in 0..num_of_pages.as_usize() {
        let page = Page::<Size4KiB>::containing_address(virt + Size4KiB::SIZE * i as u64);
        let frame = (*FRAME_ALLOCATOR.wait().unwrap())
            .inner
            .lock()
            .allocate_frame()
            .expect("Phys Memory not avialable");
        let flags =
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE;
        unsafe {
            page_table
                .map_to(page, frame, flags, FRAME_ALLOCATOR.wait().as_mut().unwrap())
                .unwrap()
                .flush();
        }
    }
}
/// TODO
pub fn allocate_new(num_of_pages: NumOfPages<Size4KiB>) -> Option<VirtAddr> {
    let virt = search_free_addr(num_of_pages)?;
    allocate_pages(virt, num_of_pages);
    Some(virt)
}

/// TODO
pub type Single<T> = ReadWrite<T, MemoryMapper>;

/// TODO
/// # Safety
pub unsafe fn new<T>(phys_base: PhysAddr) -> Single<T>
where
    T: Copy,
{
    ReadWrite::new(phys_base.as_u64().try_into().unwrap(), MemoryMapper)
}

/// TODO
pub struct MemoryMapper;

impl accessor::Mapper for MemoryMapper {
    /// Map a number of frames to a random free virtual memory address from a physical address to
    /// the size of an object
    unsafe fn map(&mut self, phys_start: usize, bytes: usize) -> core::num::NonZeroUsize {
        let phys_start = PhysAddr::new(phys_start.try_into().unwrap());
        let bytes = Bytes::new(bytes);

        let start_frame_addr = phys_start.align_down(Size4KiB::SIZE);
        let end_frame_addr = (phys_start + bytes.as_usize()).align_down(Size4KiB::SIZE);

        let num_pages = Bytes::new(usize::try_from(end_frame_addr - start_frame_addr).unwrap() + 1)
            .as_num_of_pages::<Size4KiB>();

        let virt = search_free_addr(num_pages).expect("OOM Virtual");
        let mut page_table = RecursivePageTable::new(active_level_4_table()).unwrap();

        for i in 0..num_pages.as_usize() {
            let page = Page::<Size4KiB>::containing_address(virt + Size4KiB::SIZE * i as u64);
            let frame = (*FRAME_ALLOCATOR.wait().unwrap())
                .inner
                .lock()
                .allocate_frame_nth(start_frame_addr + Size4KiB::SIZE * i as u64)
                .expect("Phys Memory not avialable");
            let flags = PageTableFlags::PRESENT
                | PageTableFlags::WRITABLE
                | PageTableFlags::USER_ACCESSIBLE;

            page_table
                .map_to(page, frame, flags, FRAME_ALLOCATOR.wait().as_mut().unwrap())
                .unwrap()
                .flush();
        }

        let page_offset = phys_start.as_u64() % Size4KiB::SIZE;

        let v = virt + page_offset;
        let v: usize = v.as_u64().try_into().unwrap();

        NonZeroUsize::new(v).expect("Failed to map pages.")
    }

    /// Unmap a number of pages based on the object size from a virtual address from there
    /// mapped physical frames
    fn unmap(&mut self, virt_start: usize, bytes: usize) {
        let virt_start = VirtAddr::new(virt_start.try_into().unwrap());
        let bytes = Bytes::new(bytes);

        let start_frame_addr = virt_start.align_down(Size4KiB::SIZE);
        let end_frame_addr = (virt_start + bytes.as_usize()).align_down(Size4KiB::SIZE);

        let num_pages = Bytes::new(usize::try_from(end_frame_addr - start_frame_addr).unwrap())
            .as_num_of_pages::<Size4KiB>();
        deallocate_pages(start_frame_addr, num_pages);
    }
}
