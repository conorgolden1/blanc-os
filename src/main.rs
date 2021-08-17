#![no_std]
#![no_main]

use bootloader::entry_point;
//  Macro for pointing to where the entry point function is
entry_point!(main);


use bootloader::BootInfo;
use printer::{print, println};

/// The kernels main after being handed off from the bootloader
///
/// This area is where the execution of the kernel begins
fn main(boot_info: &'static mut BootInfo) -> ! {

    let frame_buffer_info  = boot_info.framebuffer.as_ref().unwrap().info();
    if let Some(frame_buffer) =  boot_info.framebuffer.as_mut() {
        blanc_os::init_logger(frame_buffer.buffer_mut(), frame_buffer_info);
    }
    blanc_os::init();
    
    
    blanc_os::halt_loop()
}


use core::{panic::PanicInfo};
/// Operating System panic handler for stopping
/// execution in the face of an error
#[panic_handler]
fn panic(_info : &PanicInfo) -> ! {

    // println!("{}", _info);
    blanc_os::halt_loop()
}

