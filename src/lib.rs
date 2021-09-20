//! Blanc OS's parts are designed in modularized crates added as dependencies
//! those parts are then initalized and executed in the main or init functions
//! located here
//!
//! List of Kernel Crates
//! 1. [coop](../coop/index.html)   :  Cooperative Multitasking
//! 2. [fs](../fs/index.html)   : Virtual FileSystem
//! 3. [gdt](../fs/index.html)  : Global Descriptor Table
//! 4. [interrupts](../interrupts/index.html)  : Interrupt Descriptor Table
//! 5. [memory](../memory/index.html)  : Physical and Virtual Memory Manager
//! 6. [printer](../printer/index.html)   : Screen printing functionality
//! 7. [serial](../serial/index.html)   : Qemu emulator terminal printing functionality
//! 8. [task](../task/index.html)   : Process/Task usage and Multitasking functionality

#![no_std]
#![feature(asm)]
#![reexport_test_harness_main = "test_main"]
#![test_runner(crate::test_runner)]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![feature(alloc_error_handler)]
#[allow(unused_imports)]
use memory::allocator::{linked_list::LinkedListAllocator, Locked};

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
/// Initializes the Writer to the screen
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
    use task::task::{Ring, Task};
    Task::binary(
        Some("hello_world"),
        HELLO_WORLD,
        Some(Ring::Ring0),
        Some(0x1000_0000),
    );
}

#[test_case]
fn test_empty_load_elf2() {
    use task::task::{Ring, Task};
    Task::binary(
        Some("hello_world"),
        HELLO_WORLD,
        Some(Ring::Ring0),
        Some(0xF000_0000),
    );
}

#[test_case]
fn test_switch_to_elf() {}
#[test_case]
fn test_change_virtual_address_space() {
    use core::ops::Index;
    use memory::{swap_to_kernel_table, RECURSIVE_INDEX};
    use task::task::Pml4Creator;
    use x86_64::registers::control::Cr3;
    use x86_64::structures::paging::PhysFrame;

    let page_table = Pml4Creator::default().create();
    let pt1 = Cr3::read_raw();
    unsafe {
        Cr3::write(
            PhysFrame::containing_address(page_table.index(511).addr()),
            Cr3::read().1,
        )
    };
    *RECURSIVE_INDEX.wait().unwrap().lock() = 511;
    let pt2 = Cr3::read_raw();
    swap_to_kernel_table();
    assert_ne!(pt1, pt2)
}

#[test_case]
fn test_create_empty_page_tables() {
    use task::task::Pml4Creator;
    let _one = Pml4Creator::default().create();
    let _two = Pml4Creator::default().create();
}

#[test_case]
fn test_construct_page_table() {
    use core::ops::Index;
    use memory::{active_level_4_table, phys::FRAME_ALLOCATOR, RECURSIVE_INDEX};
    use task::task::Pml4Creator;
    use x86_64::registers::control::Cr3;
    use x86_64::structures::paging::{
        FrameAllocator, Page, PageTableFlags, PhysFrame, RecursivePageTable, Size4KiB,
    };
    use x86_64::VirtAddr;
    let page_table = Pml4Creator::default().create();
    unsafe {
        Cr3::write(
            PhysFrame::containing_address(page_table.index(511).addr()),
            Cr3::read().1,
        )
    };
    *RECURSIVE_INDEX.wait().unwrap().lock() = 511;
    let mut rpt = RecursivePageTable::new(active_level_4_table()).unwrap();
    let frame = FRAME_ALLOCATOR
        .wait()
        .unwrap()
        .allocate_frame()
        .expect("Frame cannot be allocated");
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

    for i in 0..10 {
        let addr = VirtAddr::new(0xDEAD_BEEF + (i * Size4KiB::SIZE));
        unsafe {
            rpt.map_to(
                Page::<Size4KiB>::containing_address(addr),
                frame,
                flags,
                FRAME_ALLOCATOR.wait().as_mut().unwrap(),
            )
            .unwrap()
            .ignore();
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////
//                                  Testing
////////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
use bootloader::{entry_point, BootInfo};
use x86_64::structures::paging::{Mapper, PageSize};

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        serial_print!("{}...\t", core::any::type_name::<T>());
        self();
        serial_println!("[ok]")
    }
}
