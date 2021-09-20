//! Interrupt Descriptor Table functionalitity

#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(global_asm)]
#![feature(asm)]

/// Calls the load IDT function, loading the table into the cpu
pub fn init_idt() {
    IDT.load();
}

use coop::keyboard;
use coop::mouse;
use lazy_static::lazy_static;
use printer::{print, println};
use task::scheduler::Scheduler;
use x86_64::structures::idt::InterruptDescriptorTable;

pub mod syscall;

lazy_static! {
    ///Static Interrupt Descriptor Table with all of the registered interrupt types and their handler functions
    pub static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.overflow.set_handler_fn(overflow_handler);
        unsafe {
            idt.double_fault.set_handler_fn(double_fault_handler).set_stack_index(gdt::DOUBLE_FAULT_INDEX);
        }
        idt.bound_range_exceeded.set_handler_fn(bound_range_handler);
        idt.segment_not_present.set_handler_fn(segment_not_present_handler);
        idt.alignment_check.set_handler_fn(alignment_handler);
        idt.invalid_opcode.set_handler_fn(invalid_op_handler);
        idt.invalid_tss.set_handler_fn(invalid_tss_handler);
        idt.stack_segment_fault.set_handler_fn(stack_segment_handler);
        idt.security_exception.set_handler_fn(security_exception_handler);
        for i in PIC_1_OFFSET..(PIC_2_OFFSET + 8) {
            idt[i as usize].set_handler_fn(tmp_handler);
        }

        idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);
        idt[InterruptIndex::PrimATA.as_usize()].set_handler_fn(ata_interrupt_handler);
        idt[InterruptIndex::Mouse.as_usize()].set_handler_fn(mouse_interrupt_handler);
        idt[0x80].set_handler_fn(syscall);
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt.divide_error.set_handler_fn(divide_error_handler);
        idt.general_protection_fault.set_handler_fn(general_protection_handler);
        idt.virtualization.set_handler_fn(virtualization_handler);


        idt
    };
}

extern "x86-interrupt" fn syscall(_: InterruptStackFrame) {
    //     unsafe { syscall(4, 5, 6 ,7)};

    let call_num: u64;
    let param1: u64;
    let param2: u64;
    let param3: u64;
    unsafe {
        asm!("mov {}, rax", out(reg) call_num);
        asm!("mov {}, rdi", out(reg) param1);
        asm!("mov {}, rsi", out(reg) param2);
        asm!("mov {}, rdx", out(reg) param3);
    }
    if (call_num as usize) < SYSTEM_CALLS.len() {
        SYSTEM_CALLS[call_num as usize](param1, param2, param3);
    }
    unsafe {
        PICS.lock().notify_end_of_interrupt(0x80);
    }
}

use pic8259::ChainedPics;

/// Static PICS controller wrapped in a Mutex
pub static PICS: spin::Mutex<ChainedPics> =
    spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

/// Remapped PIC 1 controller offset in the interrupt controller table
pub const PIC_1_OFFSET: u8 = 32;

/// Remapped PIC 2 controller offset in the interrupt controller table
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static READY: spin::Mutex<bool> = spin::Mutex::new(false); 

use x86_64::structures::idt::InterruptStackFrame;
use x86_64::structures::idt::SelectorErrorCode;

extern "x86-interrupt" fn general_protection_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "EXCEPTION: GENERAL PROTECTION FAULT\n{:#?}\nERROR CODE : {:#?}",
        stack_frame,
        SelectorErrorCode::new(error_code)
    );
}

extern "x86-interrupt" fn divide_error_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: DIVIDE ERROR\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn security_exception_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "EXCEPTION: SECURITY EXCEPTION ERROR\n{:#?}\nERROR CODE : {:#?}",
        stack_frame,
        SelectorErrorCode::new(error_code)
    );
}

extern "x86-interrupt" fn stack_segment_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    panic!(
        "EXCEPTION: STACK SEGMENT FAULT\n{:#?}\nERROR CODE : {:#?}",
        stack_frame,
        SelectorErrorCode::new(error_code)
    );
}

extern "x86-interrupt" fn invalid_tss_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    panic!(
        "EXCEPTION: INVALID TSS\n{:#?}\nERROR CODE : {:#?}",
        stack_frame,
        SelectorErrorCode::new(error_code)
    );
}

extern "x86-interrupt" fn invalid_op_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: INVALID OPCODE\n{:#?}\n", stack_frame);
}

extern "x86-interrupt" fn alignment_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    panic!(
        "EXCEPTION: OUT OF ALIGNMENT\n{:#?}\nERROR CODE : {:#?}",
        stack_frame,
        SelectorErrorCode::new(error_code)
    );
}

extern "x86-interrupt" fn segment_not_present_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "EXCEPTION: SEGMENT NOT PRESENT\n{:#?}\nERROR CODE : {:#?}",
        stack_frame,
        SelectorErrorCode::new(error_code)
    );
}

extern "x86-interrupt" fn bound_range_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: BOUND RANGE EXCEEDED\n{:#?}\n", stack_frame);
}

extern "x86-interrupt" fn virtualization_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: VIRT EXCEPTION\n{:#?}\n", stack_frame);
}

extern "x86-interrupt" fn tmp_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: TEMP EXCEPTION\n{:#?}\n", stack_frame);
}

extern "x86-interrupt" fn overflow_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("{:#?}", stack_frame);

    loop {
        x86_64::instructions::hlt();
    }
}

///Doesnt do anything at the moment
///TODO: Notify the ata caller that the ata controller is ready
extern "x86-interrupt" fn ata_interrupt_handler(_stack_frame: InterruptStackFrame) {
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::PrimATA.as_u8());
    }
}

///Reads the key code from 0x60 port and adds that to the keyboard task handler
extern "x86-interrupt" fn mouse_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;
    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };
    unsafe { mouse::add_scancode(scancode) };

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Mouse.as_u8());
    }
}

///Reads the key code from 0x60 port and adds that to the keyboard task handler
extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;
    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };
    keyboard::add_scancode(scancode);

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

use x86_64::registers::control::Cr2;
use x86_64::structures::idt::PageFaultErrorCode;
use x86_64::structures::paging::Mapper;
use x86_64::structures::paging::Page;
use x86_64::structures::paging::PageTableFlags;
use x86_64::structures::paging::RecursivePageTable;
use x86_64::structures::paging::Size4KiB;

use crate::syscall::SYSTEM_CALLS;

///Page fault handler prints out the respective errors and stack frame and halts cpu execution
extern "x86-interrupt" fn page_fault_handler(
    _stack_frame: InterruptStackFrame,
    _error_code: PageFaultErrorCode,
) {
    let acc_addr = Cr2::read();
    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", acc_addr);
    println!("Error Code: {:?}", _error_code);
    println!("{:#?}", _stack_frame);

    if _error_code == PageFaultErrorCode::INSTRUCTION_FETCH {
        let mut rpt = RecursivePageTable::new(memory::active_level_4_table()).unwrap();
        let page = Page::<Size4KiB>::containing_address(acc_addr);
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe {
            rpt.update_flags(page, flags).unwrap();
            PICS.lock().notify_end_of_interrupt(0xE);
        }
    } else {
        loop {
            x86_64::instructions::hlt();
        }
    }
}

///Used for task time slices
extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    println!("*");
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
    if *READY.lock() {
        Scheduler::run();
    }
    

}

///Double fault interrupt panics and prints the stack frame
extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

///Breakpoints print out the stack frame at a specified breakpoint
extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame)
}

/// Interrupt Index enum with all of the different interrupt handler types  
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
    Cascade,
    COM2,
    COM1,
    LPT2,
    FloppyDisk,
    LPT1,
    CmosRealTimeClock,
    ACPI,
    Free1,
    Free2,
    Mouse,
    Coprocessor,
    PrimATA,
    SecoATA,
}

impl InterruptIndex {
    /// u8 representation from the PIC_1_OFFSET
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// usize representation from the PIC_1_OFFSET
    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}
