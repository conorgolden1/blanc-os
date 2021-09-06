#![no_std]
#![no_main]
use core::panic::PanicInfo;

use printer::{println, print};
use serial::{serial_println, serial_print};

extern "C" fn _start() -> ! {
    println!("Hello, world!");

    loop {

    }
}


#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        
    }
}