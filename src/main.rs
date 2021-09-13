#![no_std]
#![no_main]
#![feature(asm)]
#![feature(custom_test_frameworks)]
#![test_runner(test_runner)]
#![reexport_test_harness_main = "test_main"]

use alloc::boxed::Box;
use alloc::vec;
#[allow(unused_imports)]
use blanc_os::test_runner;

extern crate alloc;

use bootloader::entry_point;

//  Macro for pointing to where the entry point function is
entry_point!(main);

// KERNEL START : 0xFFFF800000000000
// HEAP START   : 0xFFFF_F000_0000_0000
// HEAP SIZE    : 100 * 1024
// STACK START  : 0xFFFF_F000_0001_9000
// STACK SIZE   : 80 * 1024

use bootloader::BootInfo;
use coop::executor::Executor;
use coop::keyboard;
use coop::mouse;
use coop::Task;

use interrupts::syscall;
use memory::active_level_4_table;
use memory::KERNEL_PAGE_TABLE;

use memory::allocator;
use memory::init;

use memory::phys::PhysFrameAllocator;
use memory::phys::FRAME_ALLOCATOR;

use printer::{print, println};

use serial::serial_print;
use serial::serial_println;
use task::elf2::load_elf;
// use task::context_switch::new_context_switch;
// use task::elf;
// use task::elf::Pml4Creator;
// use x86_64::PhysAddr;
use x86_64::registers::control::Cr3;
use x86_64::registers::control::Cr4;
use x86_64::structures::paging::FrameAllocator;
use x86_64::structures::paging::Mapper;
use x86_64::structures::paging::Size4KiB;

#[rustfmt::skip]
static HELLO_WORLD: &[u8] = include_bytes!("../applications/hello_world/target/hello_world/debug/hello_world");


/// The kernels main after being handed off from the bootloader
///
/// This area is where the execution of the kernel begins
fn main(boot_info: &'static mut BootInfo) -> ! {
    let frame_buffer_info = boot_info.framebuffer.as_ref().unwrap().info();
    if let Some(frame_buffer) = boot_info.framebuffer.as_mut() {
        blanc_os::init_logger(frame_buffer.buffer_mut(), frame_buffer_info);
    }

    blanc_os::init();

    unsafe { init(boot_info.recursive_index) };

    PhysFrameAllocator::init(&boot_info.memory_regions);

    allocator::init_heap().expect("Heap did not properly map");

    #[cfg(test)]
    test_main();
    
    use task::elf2::align_bin;

    let raw = align_bin(HELLO_WORLD);
    let elf = load_elf(raw.as_slice(), 0xFF00_0000);
    unsafe {
    x86_64::registers::control::Efer::write_raw(
        x86_64::registers::control::Efer::read_raw() ^ 2^11);
    asm!(
        "jmp {}",
        in(reg) (elf.entry_point() + 0xFF00_0000)
    );}


    let mut executor = Executor::new();

    executor.spawn(Task::new(keyboard::print_keypresses()));
    executor.spawn(Task::new(mouse::print_mouse()));
    executor.run();
}

use core::panic::PanicInfo;

/// Operating System panic handler for stopping
/// execution in the face of an error

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    println!("{}", _info);
    blanc_os::halt_loop()
}

/////////////////////////////////////////////////////////////////
//                          TESTS
////////////////////////////////////////////////////////////////

#[test_case]
fn test_main_testing() {
    assert_eq!(2 + 2, 4)
}

#[test_case]
fn test_same_frame_alloc_dealloc() {
    use memory::phys::FRAME_ALLOCATOR;
    use x86_64::structures::paging::{FrameAllocator, FrameDeallocator};

    let mut frame_allocator = FRAME_ALLOCATOR.wait().unwrap();
    for _ in 0..100 {
        let x = frame_allocator.allocate_frame().unwrap();
        unsafe { frame_allocator.deallocate_frame(x) };
        let y = frame_allocator.allocate_frame().unwrap();
        assert_eq!(x, y);
    }
}

#[test_case]
fn test_new_frame_alloc() {
    use memory::phys::FRAME_ALLOCATOR;
    use x86_64::structures::paging::FrameAllocator;

    let mut frame_allocator = FRAME_ALLOCATOR.wait().unwrap();
    assert_ne!(
        frame_allocator.allocate_frame(),
        frame_allocator.allocate_frame()
    )
}

#[test_case]
fn test_print() {
    print!("")
}

#[test_case]
fn test_println() {
    println!()
}

#[test_case]
fn test_box_heap_alloc() {
    use alloc::boxed::Box;

    drop(Box::new([0u64; 100]));
}

#[test_case]
fn test_vec_heap_alloc() {
    use alloc::vec::Vec;

    let mut vec: Vec<u64> = Vec::new();
    for i in 0..50 {
        vec.push(i);
    }
    drop(vec)
}

#[test_case]
fn test_allocated_virtual_address() {
    use memory::phys::FRAME_ALLOCATOR;
    use x86_64::structures::paging::Page;
    use x86_64::structures::paging::PageTableFlags;
    use x86_64::structures::paging::RecursivePageTable;
    use x86_64::VirtAddr;

    let mut current_pt =
        RecursivePageTable::new(active_level_4_table()).expect("Couldn't obtain active page table");

    let addr = VirtAddr::new(0xdeadbeef);
    let page = Page::<Size4KiB>::containing_address(addr);
    let frame = FRAME_ALLOCATOR.wait().unwrap().allocate_frame().unwrap();
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

    unsafe { current_pt.map_to(page, frame, flags, FRAME_ALLOCATOR.wait().as_mut().unwrap()) };

    // Assert that the memory address is not available
    assert!(!memory::virt::available(addr))
}


#[test_case]
fn test_syscall_print() {
    use interrupts::syscall::syscall;
    let hello_world = "hello world";
    let hello_world_ptr = hello_world.as_ptr() as u64;
    let num_bytes = hello_world.as_bytes().len();
    unsafe { syscall(0, 0, hello_world_ptr ,num_bytes as u64)};
}

