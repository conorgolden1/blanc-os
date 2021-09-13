#![no_std]
#![no_main]
#![feature(asm)]
#![feature(abi_x86_interrupt)]
#![feature(custom_test_frameworks)]
#![test_runner(test_runner)]
#![reexport_test_harness_main = "test_main"]

use blanc_os::test_runner;
use coop::{Task, executor::Executor};
use memory::{allocator, phys::PhysFrameAllocator};

use core::panic::PanicInfo;
use serial::{serial_print, serial_println};

use bootloader::{BootInfo, entry_point};

//  Macro for pointing to where the entry point function is
entry_point!(main);

fn main(boot_info: &'static mut BootInfo) -> ! {
    let frame_buffer_info = boot_info.framebuffer.as_ref().unwrap().info();
    if let Some(frame_buffer) = boot_info.framebuffer.as_mut() {
        blanc_os::init_logger(frame_buffer.buffer_mut(), frame_buffer_info);
    }

    blanc_os::init();

    unsafe { memory::init(boot_info.recursive_index) };

    PhysFrameAllocator::init(&boot_info.memory_regions);

    allocator::init_heap().expect("Heap did not properly map");

    #[cfg(test)]
    test_main();
    
    


    

    let mut executor = Executor::new();

    executor.spawn(Task::new(coop::keyboard::print_keypresses()));
    executor.spawn(Task::new(coop::mouse::print_mouse()));
    executor.run();

}


#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    blanc_os::test_panic_handler(info)
}
