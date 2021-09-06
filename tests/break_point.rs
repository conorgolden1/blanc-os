#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(custom_test_frameworks)]
#![test_runner(test_runner)]
#![reexport_test_harness_main = "test_main"]

use blanc_os::test_runner;

use core::panic::PanicInfo;
use serial::{serial_print, serial_println};
#[no_mangle]
pub extern "C" fn _start() -> ! {
    serial_print!("break_point::break_point...\t");

    gdt::init();
    init_test_idt();
    breakpoint();
    panic!("Execution continued after break_point");
}


fn breakpoint() {
    x86_64::instructions::interrupts::int3();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    blanc_os::test_panic_handler(info)
}

use lazy_static::lazy_static;
use x86_64::structures::idt::InterruptDescriptorTable;

lazy_static! {
    static ref TEST_IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint
            .set_handler_fn(test_break_trap_handler);
        idt
    };
}

pub fn init_test_idt() {
    TEST_IDT.load();
}



use blanc_os::{exit_qemu, QemuExitCode};
use x86_64::structures::idt::InterruptStackFrame;

extern "x86-interrupt" fn test_break_trap_handler(
    _stack_frame: InterruptStackFrame,
) {
    serial_println!("[ok]");
    exit_qemu(QemuExitCode::Success);
    loop {
        
    }
}