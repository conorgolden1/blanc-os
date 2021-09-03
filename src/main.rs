#![no_std]
#![no_main]
#![feature(asm)]

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;
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

use memory::active_level_4_table;
use memory::allocator;
use memory::init;
use memory::phys::PhysFrameAllocator;

use printer::{print, println};
use task::context_switch::new_context_switch;
use task::elf::Pml4Creator;
use x86_64::PhysAddr;
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::PageTable;




#[rustfmt::skip]
static USERLAND_SHELL: &[u8] = include_bytes!("shell");

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


    // for (i, entry) in KERNEL_PAGE_TABLE.wait().unwrap().lock().level_4_table().iter().enumerate() {
    //     if !entry.is_unused() {
    //         println!("{} {:#?}", i , entry);
    //     }
    // }




    let shell_proc = task::binary("shell", USERLAND_SHELL, task::task::Ring::Ring3);



    // for (i, entry)  in shell_proc.tables.pml4.iter().enumerate() {
    //     if !entry.is_unused() {
    //         println!("{}, {:#?}", i,  entry);
    //     }
    // }
  
    // for (i, entry) in shell_proc.tables.pml4.iter().enumerate() {
    //     if !entry.is_unused() {
    //         println!("{} {:#?}", i, entry)
    //     }
    // }
    // x86_64::instructions::interrupts::disable();
    unsafe {
        new_context_switch(shell_proc.pml4,shell_proc.stack_frame_top_addr(), shell_proc.entry);
    }
    
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

// TODO ADD INTO TEST
// loop {
//     let x = frame_allocator.allocate_frame().unwrap();
//     unsafe { frame_allocator.deallocate_frame(x) };
//     let y = frame_allocator.allocate_frame().unwrap();
//     assert_eq!(x, y);
// }

// // TODO ADD INTO TEST
// assert_ne!(FRAME_ALLOCATOR.wait().unwrap().allocate_frame(), FRAME_ALLOCATOR.wait().unwrap().allocate_frame());
    // let mut x = aside_virt::address_space::AddressSpace::new().unwrap();

    // RAMFS.root_inode.mkdir("foo.txt").unwrap();
    // print_filesystem();
    // let mut task = task::Task::new_idle();
    // let mut tmm = TaskMemoryMap::new();

    // let shell_elf = ElfFile::new(memory::USERLAND_SHELL).unwrap();
    // shell_elf.tmm.load_bin(&shell_elf);
    // task.exec(tmm, &shell_elf);