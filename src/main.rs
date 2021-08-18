#![no_std]
#![no_main]

use bootloader::entry_point;
//  Macro for pointing to where the entry point function is
entry_point!(main);


use bootloader::BootInfo;
use memory::PhysFrameAllocator;
use memory::init;
use printer::{print, println};
use x86_64::structures::paging::FrameAllocator;
use x86_64::structures::paging::FrameDeallocator;

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
    let mut frame_allocator = unsafe {PhysFrameAllocator::init(&boot_info.memory_regions, &mut page_table) };

    allocator::init_heap(&mut page_table, &mut frame_allocator).expect("Heap did not properly map");
    
    let x = frame_allocator.allocate_frame().unwrap();
    unsafe { frame_allocator.deallocate_frame(x) };
    let y = frame_allocator.allocate_frame().unwrap();
    println!("{:#?} {:#?} {}", x, y, x == y);
    assert_eq!(x, y);
    
    
    blanc_os::halt_loop()

}


use core::{panic::PanicInfo};
/// Operating System panic handler for stopping
/// execution in the face of an error
#[panic_handler]
fn panic(_info : &PanicInfo) -> ! {
    println!("{}", _info);
    blanc_os::halt_loop()
}

