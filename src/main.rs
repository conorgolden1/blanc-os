#![no_std]
#![no_main]
#![feature(asm)]

extern crate alloc;

use alloc::vec;
use bootloader::entry_point;
//  Macro for pointing to where the entry point function is
entry_point!(main);

// KERNEL START : 0xFFFF800000000000
// HEAP START   : 0xFFFF_F000_0000_0000
// HEAP SIZE    : 100 * 1024
// STACK START  : 0xFFFF_F000_0001_9000
// STACK SIZE   : 80 * 1024

use bootloader::BootInfo;
use coop::Task;
use coop::executor::Executor;
use coop::keyboard;
use coop::mouse;
use fs::inode::INode;
use fs::inode::OFlags;
use fs::ramdisk;
use fs::ramdisk::RAMFS;
use fs::ramdisk::print_filesystem;
use gdt::GDT;
use interrupts::InterruptIndex;
use interrupts::PICS;
use memory::phys::FRAME_ALLOCATOR;
use memory::phys::PhysFrameAllocator;
use memory::init;
use memory::virt;
use printer::{print, println};
use serial::serial_println;

use x86_64::PhysAddr;
use x86_64::VirtAddr;
use x86_64::structures::paging::FrameAllocator;
use x86_64::structures::paging::FrameDeallocator;
use x86_64::structures::paging::Translate;

/// The kernels main after being handed off from the bootloader
///
/// This area is where the execution of the kernel begins
fn main(boot_info: &'static mut BootInfo) -> ! {


    let frame_buffer_info  = boot_info.framebuffer.as_ref().unwrap().info();
    if let Some(frame_buffer) =  boot_info.framebuffer.as_mut() {
        blanc_os::init_logger(frame_buffer.buffer_mut(), frame_buffer_info);
    }
    
    blanc_os::init();

    let mut page_table = unsafe { init(boot_info.physical_memory_offset) };
    PhysFrameAllocator::init(&boot_info.memory_regions, &mut page_table);
    
    allocator::init_heap(&mut page_table).expect("Heap did not properly map");

    let mut x = virt::address_space::AddressSpace::new().unwrap();

    RAMFS.root_inode.mkdir("foo.txt").unwrap();
    print_filesystem();

    let mut executor = Executor::new();

    executor.spawn(Task::new(keyboard::print_keypresses()));
    executor.spawn(Task::new(mouse::print_mouse()));
    executor.run();
    
}


use core::{panic::PanicInfo};
/// Operating System panic handler for stopping
/// execution in the face of an error
#[panic_handler]
fn panic(_info : &PanicInfo) -> ! {
    println!("{}", _info);
    blanc_os::halt_loop()
}

// TODO ADD INTO TEST
// loop {
//     let x = frame_allocator.allocate_frame().unwrap();
//     unsafe { frame_allocator.deallocate_frame(x) };
//     let y = frame_allocator.allocate_frame().unwrap();
//     assert_eq!(x, y);
// }

// // TODO ADD INTO TEST
// assert_ne!(FRAME_ALLOCATOR.wait().unwrap().allocate_frame(), FRAME_ALLOCATOR.wait().unwrap().allocate_frame());