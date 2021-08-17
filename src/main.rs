#![no_std]
#![no_main]

use serial::{serial_println};
use bootloader::entry_point;
//  Macro for pointing to where the entry point function is
entry_point!(main);

use bootloader::BootInfo;
/// The kernels main after being handed off from the bootloader
///
/// This area is where the execution of the kernel begins
fn main(_boot_info: &'static mut BootInfo) -> ! {
    blanc_os::init();
    serial_println!("{:#?}", _boot_info);
    blanc_os::halt_loop()
}


use core::panic::PanicInfo;
/// Operating System panic handler for stopping
/// execution in the face of an error
#[panic_handler]
fn panic(_info : &PanicInfo) -> ! {

    // println!("{}", _info);
    blanc_os::halt_loop()
}

