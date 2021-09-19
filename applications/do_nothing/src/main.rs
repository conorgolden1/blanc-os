#![no_main]
#![no_std]
#![feature(lang_items)]
#![allow(dead_code)]

use core::panic::PanicInfo;



#[no_mangle] // don't mangle the name of this function
pub extern "C" fn _start() -> ! {
    loop {}
}



#[panic_handler]
fn panic(_panic: &PanicInfo<'_>) -> ! {
    loop {}
}
