#![no_std]
#![no_main]
#![feature(asm)]
#![feature(custom_test_frameworks)]
#![test_runner(test_runner)]
#![reexport_test_harness_main = "test_main"]

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


use memory::KERNEL_PAGE_TABLE;
use memory::active_level_4_table;

use memory::allocator;
use memory::init;

use memory::phys::PhysFrameAllocator;

use printer::{print, println};

// use task::context_switch::new_context_switch;
// use task::elf;
// use task::elf::Pml4Creator;
// use x86_64::PhysAddr;
use x86_64::registers::control::Cr3;





#[rustfmt::skip]
static USERLAND_SHELL: &[u8] = include_bytes!("../applications/shell/target/x86_64-rust-os/debug/shell");



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

    // for (i, entry) in KERNEL_PAGE_TABLE.wait().unwrap().lock().level_4_table().iter().enumerate() {
    //     if !entry.is_unused() {
    //         println!("{} {:#?}", i , entry);
    //     }
    // }
    
    println!("KERNEL_PAGE_TABLE {:#?}", KERNEL_PAGE_TABLE.wait().unwrap().lock().level_4_table().index(0).addr());
    assert_eq!(KERNEL_PAGE_TABLE.wait().unwrap().lock().level_4_table().index(0).frame().unwrap(), active_level_4_table().index(0).frame().unwrap());
    assert_eq!(KERNEL_PAGE_TABLE.wait().unwrap().lock().level_4_table().index(508).frame().unwrap(), Cr3::read().0);
    
    let x = xmas_elf::ElfFile::new(USERLAND_SHELL).unwrap();
    println!("RAW ENTRY POINT {:#?}", x.header.pt2.entry_point());

    let _shell_proc = task::binary("shell", USERLAND_SHELL, task::task::Ring::Ring3);
    
    // for (i, (entry_x, entry_y)) in shell_proc.pml4.iter().zip(KERNEL_PAGE_TABLE.wait().unwrap().lock().level_4_table().iter()).enumerate(){
    //     if !entry_x.is_unused() {
    //         println!("{} {:#?}", i , entry_x);
    //         assert!(!entry_y.is_unused());
    //     }
    // }
    
  
    
    // x86_64::instructions::interrupts::disable();
    // println!("SWITCHING TO {}!", shell_proc.name);
    // unsafe {
    //     new_context_switch(shell_proc);
    // }
    
    let mut executor = Executor::new();

    executor.spawn(Task::new(keyboard::print_keypresses()));
    executor.spawn(Task::new(mouse::print_mouse()));
    executor.run();
}



use core::ops::Index;
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
    assert_eq!(2+2, 4)
}

#[test_case]
fn test_same_frame_alloc_dealloc() {
    use memory::phys::FRAME_ALLOCATOR;
    use x86_64::structures::paging::{FrameDeallocator, FrameAllocator};

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
    assert_ne!(frame_allocator.allocate_frame(), frame_allocator.allocate_frame())
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

