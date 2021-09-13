#![no_std]
#![feature(asm)]
#![reexport_test_harness_main = "test_main"]
#![test_runner(crate::test_runner)]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]

#![feature(alloc_error_handler)]
#[allow(unused_imports)]
use memory::allocator::{Locked, linked_list::LinkedListAllocator};



/// 1. Initialize the global descriptor table
/// 2. Initialize the interrupt descriptor table
/// 3. Initialize the Programmable Interrupt Controller
/// 4. Enable CPU interrupts
pub fn init() {
    gdt::init();
    interrupts::init_idt();
    unsafe { interrupts::PICS.lock().initialize() };
    x86_64::instructions::interrupts::enable();
}

///Halts the CPU on a loop without return
pub fn halt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

use bootloader::boot_info::FrameBufferInfo;


use printer::{Writer, WRITER};
use serial::{serial_print, serial_println};
use spin::Mutex;

pub fn init_logger(framebuffer: &'static mut [u8], info: FrameBufferInfo) {
    let mutex_writer = Mutex::new(Writer::new(framebuffer, info));
    WRITER.init_once(|| mutex_writer);
}

////////////////////////////////////////////////////////////////////////////////////
//                                  Tests
////////////////////////////////////////////////////////////////////////////////////

#[test_case]
fn test_lib_testing() {
    assert_eq!(1 + 1, 2)
}

#[cfg(test)]
#[rustfmt::skip]
static HELLO_WORLD: &[u8] = include_bytes!("../applications/hello_world/target/hello_world/debug/hello_world");

#[test_case]
fn test_empty_load_elf() {
    use task::elf2::load_elf;
    use task::elf2::align_bin;

    let raw = align_bin(HELLO_WORLD);
    load_elf(raw.as_slice(), 0x1000_0000);
}

#[test_case]
fn test_empty_load_elf2() {
    use task::elf2::load_elf;
    use task::elf2::align_bin;

    let raw = align_bin(HELLO_WORLD);
    load_elf(raw.as_slice(), 0xF000_0000);
}

#[test_case]
fn test_switch_to_elf() {

}


////////////////////////////////////////////////////////////////////////////////////
//                                  Testing
////////////////////////////////////////////////////////////////////////////////////


#[cfg(test)]
use bootloader::{BootInfo, entry_point};

//Change the entry point if in testing mode
#[cfg(test)]
entry_point!(test_entry_main);


/// Entry point for `cargo test`
#[cfg(test)]
#[no_mangle]
fn test_entry_main(boot_info: &'static mut BootInfo) -> ! {
    let frame_buffer_info = boot_info.framebuffer.as_ref().unwrap().info();
    if let Some(frame_buffer) = boot_info.framebuffer.as_mut() {
        init_logger(frame_buffer.buffer_mut(), frame_buffer_info);
    }
    init();
    unsafe { memory::init(boot_info.recursive_index) };
    
    memory::phys::PhysFrameAllocator::init(&boot_info.memory_regions);
    
    memory::allocator::init_heap().expect("Heap did not properly map");

    test_main();
    
    halt_loop();
}


/// Testing panic handler prints out the error to the shell and closes
/// Qemu
pub fn test_panic_handler(info: &PanicInfo) -> ! {
    serial_println!("Error: {}\n", info);
    exit_qemu(QemuExitCode::Failed);
    halt_loop();
}

/// Used for testing on wether the execution was successful or has failed
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

/// Writes to the qemu pci port in emulation the exit code
pub fn exit_qemu(exit_code: QemuExitCode) {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}

use core::panic::PanicInfo;
/// Runs all of the test case functions and then exits Qemu
pub fn test_runner(tests: &[&dyn Testable]) {

    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
    exit_qemu(QemuExitCode::Success)
}

/// Testing panic caller
#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info)
}


///Trait used for printing to the environments shell
pub trait Testable {
    fn run(&self) -> ();
}


/// Formatting for printing the test to the shell
impl<T> Testable for T where T: Fn(), {
    fn run(&self) {
        serial_print!("{}...\t", core::any::type_name::<T>());
        self();
        serial_println!("[ok]")
    }
}

