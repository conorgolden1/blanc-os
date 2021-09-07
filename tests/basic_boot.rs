#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(blanc_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    
    test_main();

    loop {}
    
}


#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use blanc_os::test_panic_handler;
    test_panic_handler(info)
}

use serial::serial_println;
use serial::serial_print;


#[test_case]
fn test_serial_println() {
    serial_println!("test_serial_println output");
}


#[test_case]
fn test_serial_print() {
    serial_print!("test_serial_print output");
}