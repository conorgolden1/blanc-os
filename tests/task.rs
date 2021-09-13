#![no_std]
#![no_main]
#![feature(asm)]
#![feature(abi_x86_interrupt)]
#![feature(custom_test_frameworks)]
#![test_runner(test_runner)]
#![reexport_test_harness_main = "test_main"]

use blanc_os::test_runner;
use coop::{Task, executor::Executor};
use memory::{allocator, phys::PhysFrameAllocator};

use core::panic::PanicInfo;
use serial::{serial_print, serial_println};

use bootloader::{BootInfo, entry_point};

//  Macro for pointing to where the entry point function is
entry_point!(main);

static HELLO_WORLD: &[u8] = include_bytes!("../applications/hello_world/target/hello_world/debug/hello_world");

fn main(boot_info: &'static mut BootInfo) -> ! {
    let frame_buffer_info = boot_info.framebuffer.as_ref().unwrap().info();
    if let Some(frame_buffer) = boot_info.framebuffer.as_mut() {
        blanc_os::init_logger(frame_buffer.buffer_mut(), frame_buffer_info);
    }

    blanc_os::init();

    unsafe { memory::init(boot_info.recursive_index) };

    PhysFrameAllocator::init(&boot_info.memory_regions);

    allocator::init_heap().expect("Heap did not properly map");

    #[cfg(test)]
    test_main();
    


    

    let mut executor = Executor::new();

    executor.spawn(Task::new(coop::keyboard::print_keypresses()));
    executor.spawn(Task::new(coop::mouse::print_mouse()));
    executor.run();

}


#[test_case]
fn test_jump_to_elf() {
    use task::elf2::load_elf;
    use task::elf2::align_bin;

    let raw = align_bin(HELLO_WORLD);
    let elf = load_elf(raw.as_slice(), 0xFF00_0000);
    serial_println!("{:#?}", elf.entry_point());
    unsafe {
    x86_64::registers::control::Efer::write_raw(
        x86_64::registers::control::Efer::read_raw() ^ 2^11);
    asm!(
        "jmp {}",
        in(reg) (elf.entry_point() + 0xFF00_0000)
    );}

}


#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    blanc_os::test_panic_handler(info)
}




