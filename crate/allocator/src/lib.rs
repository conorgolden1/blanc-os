#![no_std]
#![feature(alloc_error_handler)]
#![feature(const_mut_refs)]

pub const HEAP_START: usize = 0xFFFF_F000_0000_0000;
pub const HEAP_SIZE: usize = 100 * 1024; // 100 KiB

extern crate alloc;

pub mod linked_list;

use linked_list::LinkedListAllocator;
use memory::frame::FRAME_ALLOCATOR;

#[global_allocator]
static ALLOCATOR: Locked<LinkedListAllocator> = Locked::new(LinkedListAllocator::new());




use x86_64::{structures::paging::{ mapper::MapToError, FrameAllocator,Mapper,Page,PageTableFlags, Size4KiB}, VirtAddr,};

pub fn init_heap(page_table: &mut impl Mapper<Size4KiB>) -> Result<(), MapToError<Size4KiB>> {
    let page_range = {
        let heap_start = VirtAddr::new(HEAP_START as u64);
        let heap_end = heap_start + HEAP_SIZE - 1u64;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_page_end = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_page_end)
    };

    for page in page_range {
        let frame = FRAME_ALLOCATOR.wait().unwrap().lock().allocate_frame().ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe {
            page_table.map_to(page, frame, flags, &mut FRAME_ALLOCATOR.wait().unwrap().lock())?.flush()
        };
    }

    unsafe {
        ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
    }
    Ok(())
}


pub struct Locked<A> {
    inner: spin::Mutex<A>
}

impl <A> Locked<A> {
    pub const fn new (inner: A) -> Self {
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




#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}