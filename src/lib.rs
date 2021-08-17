#![no_std]

/// 1. Initialize the global descriptor table
/// 2. Initialize the interrupt descriptor table
/// 3. Initialize the Programmable Interrupt Controller
/// 4. Enable CPU interrupts
pub fn init() {
    gdt::init();
    interrupts::init_idt();
}

///Halts the CPU on a loop without return
pub fn halt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}