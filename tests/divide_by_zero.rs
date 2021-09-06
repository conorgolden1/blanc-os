#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(custom_test_frameworks)]
#![feature(asm)]
#![test_runner(test_runner)]
#![reexport_test_harness_main = "test_main"]


use blanc_os::test_runner;

use core::{panic::PanicInfo};
use serial::{serial_print, serial_println};
#[no_mangle]
pub extern "C" fn _start() -> ! {
    serial_print!("divide_by_zero::divide_by_zero...\t");

    gdt::init();
    init_test_idt();
    divide_bye_zero();
    panic!("Execution continued after divide by zero");
}

#[allow(unconditional_panic)]
fn divide_bye_zero() {
    unsafe {
        asm! {
        "mov edx, 0; 
         mov eax, 1;
         mov ecx, 0;
         div ecx"
    }}
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
        idt.divide_error
            .set_handler_fn(test_zero_trap_handler);
               
        
        idt
    };
}

pub fn init_test_idt() {
    TEST_IDT.load();
}



use blanc_os::{exit_qemu, QemuExitCode};
use x86_64::structures::idt::InterruptStackFrame;

extern "x86-interrupt" fn test_zero_trap_handler(
    _stack_frame: InterruptStackFrame,
)  {
    serial_println!("[ok]");
    exit_qemu(QemuExitCode::Success);
    loop {

    }
}