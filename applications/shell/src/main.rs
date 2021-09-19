#![no_main]
#![no_std]
#![feature(lang_items)]
#![feature(global_asm)]
#![allow(dead_code)]

use core::panic::PanicInfo;

global_asm!(include_str!("syscall_interrupts.s"));


#[no_mangle] // don't mangle the name of this function
pub extern "C" fn _start() -> ! {

}

extern "C" {
    fn syscall(call_num : u64, param1 : u64, param2 : u64, param3: u64) -> u64;
}

#[panic_handler]
fn panic(_panic: &PanicInfo<'_>) -> ! {
    loop {}
}