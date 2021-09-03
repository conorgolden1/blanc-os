#![no_std]

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
use spin::Mutex;

pub fn init_logger(framebuffer: &'static mut [u8], info: FrameBufferInfo) {
    let mutex_writer = Mutex::new(Writer::new(framebuffer, info));
    WRITER.init_once(|| mutex_writer);
}
