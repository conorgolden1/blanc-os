#![no_std]
#![no_main]

use bootloader::entry_point;
//  Macro for pointing to where the entry point function is
entry_point!(main);

use bootloader::BootInfo;
/// The kernels main after being handed off from the bootloader
///
/// This area is where the execution of the kernel begins
fn main(_boot_info: &'static mut BootInfo) -> ! {
    loop {

    }
}


use core::panic::PanicInfo;
/// Operating System panic handler for stopping
/// execution in the face of an error
#[panic_handler]
fn panic(_info : &PanicInfo) -> ! {
    // Just loops for now
    loop {
        
    }
    // println!("{}", _info);
    // OS::halt_loop();
}