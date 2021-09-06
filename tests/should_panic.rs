#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use blanc_os::{QemuExitCode, exit_qemu};
use serial::{serial_print, serial_println};

use blanc_os::test_runner;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    serial_print!("[ok]");
    exit_qemu(QemuExitCode::Success);
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    should_fail();
    serial_println!("[test did not panic]");
    exit_qemu(QemuExitCode::Failed);
    loop {
         
    }
}

fn should_fail() {
    serial_print!("should_panic::should_fail...\t");
    assert_eq!(0, 1);
}