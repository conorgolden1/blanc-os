pub const HEAP_START: usize = 0xFFFF_FF00_004A_0000;

pub const HEAP_SIZE: usize = 200 * 1024; // 200 KiB

extern crate alloc;

pub mod linked_list;

use crate::{phys::FRAME_ALLOCATOR, KERNEL_PAGE_TABLE};
use linked_list::LinkedListAllocator;

#[global_allocator]
pub static ALLOCATOR: Locked<LinkedListAllocator> = Locked::new(LinkedListAllocator::new());

use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB,
    },
    VirtAddr,
};

pub fn init_heap() -> Result<(), MapToError<Size4KiB>> {
    let page_range = {
        let heap_start = VirtAddr::new(HEAP_START as u64);
        let heap_end = heap_start + HEAP_SIZE - 1u64;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_page_end = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_page_end)
    };

    for page in page_range {
        let frame = FRAME_ALLOCATOR
            .wait()
            .unwrap()
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

        unsafe {
            KERNEL_PAGE_TABLE
                .wait()
                .unwrap()
                .lock()
                .map_to(page, frame, flags, FRAME_ALLOCATOR.wait().as_mut().unwrap())?
                .flush()
        };
    }

    unsafe {
        ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
    }
    Ok(())
}

pub struct Locked<A> {
    inner: spin::Mutex<A>,
}

impl<A> Locked<A> {
    pub const fn new(inner: A) -> Self {
        Locked {
            inner: spin::Mutex::new(inner),
        }
    }

    pub fn lock(&self) -> spin::MutexGuard<A> {
        self.inner.lock()
    }
}

fn align_up(addr: usize, align: usize) -> usize {
    let remainder = addr % align;
    if remainder == 0 {
        addr
    } else {
        addr - remainder + align
    }
}
