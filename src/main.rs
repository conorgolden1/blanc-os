#![no_std]
#![no_main]
#![feature(asm)]

use bootloader::entry_point;
//  Macro for pointing to where the entry point function is
entry_point!(main);

// KERNEL START : 0xFFFF800000000000
// HEAP START   : 0xFFFF_F000_0000_0000
// HEAP SIZE    : 100 * 1024
// STACK START  : 0xFFFF_F000_0001_9000
// STACK SIZE   : 80 * 1024

use bootloader::BootInfo;
use gdt::GDT;
use memory::frame::FRAME_ALLOCATOR;
use memory::frame::PhysFrameAllocator;
use memory::init;
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

    let mut page_table = unsafe { init(boot_info.recursive_index) };
    unsafe { PhysFrameAllocator::init(&boot_info.memory_regions, &mut page_table) };

    allocator::init_heap(&mut page_table, &mut FRAME_ALLOCATOR).expect("Heap did not properly map");
    
    serial_println!("hello!");
        // TODO ADD INTO TEST
    // loop {
    //     let x = frame_allocator.allocate_frame().unwrap();
    //     unsafe { frame_allocator.deallocate_frame(x) };
    //     let y = frame_allocator.allocate_frame().unwrap();
    //     assert_eq!(x, y);
    // }


    let rbp: u64;
    unsafe {
        asm!("lea {}, [rbp]", out(reg) rbp, options(nomem, preserves_flags));
    };
    let x = x86_64::registers::control::Cr4::read();
    println!("{:#?}", x86_64::registers::control::Cr4::read_raw() );

    let y = x.bitor(x86_64::registers::control::Cr4Flags::PROTECTION_KEY );
    assert_ne!(x, y);
    unsafe {x86_64::registers::control::Cr4::write(y)};
    // 0xFFFF800000000000
    println!("{:#?}", x86_64::registers::control::Cr4::read_raw());
    blanc_os::halt_loop();
}


use core::ops::BitOr;
use core::{panic::PanicInfo};
/// Operating System panic handler for stopping
/// execution in the face of an error
#[panic_handler]
fn panic(_info : &PanicInfo) -> ! {
    println!("{}", _info);
    blanc_os::halt_loop()
}

