//! Interrupt Descriptor Table functionalitity

#![no_std]
#![feature(abi_x86_interrupt)]

/// Calls the load IDT function, loading the table into the cpu
pub fn init_idt() { 
    IDT.load();
}

use coop::keyboard;
use coop::mouse;
use lazy_static::lazy_static;
use x86_64::structures::idt::InterruptDescriptorTable;
use printer::{print, println};

lazy_static! {
    ///Static Interrupt Descriptor Table with all of the registered interrupt types and their handler functions
    pub static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
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
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt.divide_error.set_handler_fn(divide_error_handler);
        idt.general_protection_fault.set_handler_fn(general_protection_handler);
        idt.virtualization.set_handler_fn(virtualization_handler);
    

        idt
    };
}

use pic8259::ChainedPics;

/// Static PICS controller wrapped in a Mutex
pub static PICS: spin::Mutex<ChainedPics> = spin::Mutex::new(
    unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET)});


/// Remapped PIC 1 controller offset in the interrupt controller table
pub const PIC_1_OFFSET: u8 = 32;


/// Remapped PIC 2 controller offset in the interrupt controller table
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;


use x86_64::structures::idt::InterruptStackFrame;
use x86_64::structures::idt::SelectorErrorCode;

extern "x86-interrupt" fn general_protection_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    panic!("EXCEPTION: GENERAL PROTECTION FAULT\n{:#?}\nERROR CODE : {:#?}", stack_frame, SelectorErrorCode::new(error_code));
}

extern "x86-interrupt" fn divide_error_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: DIVIDE ERROR\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn security_exception_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    panic!("EXCEPTION: SECURITY EXCEPTION ERROR\n{:#?}\nERROR CODE : {:#?}", stack_frame, SelectorErrorCode::new(error_code));
}

extern "x86-interrupt" fn stack_segment_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    panic!("EXCEPTION: STACK SEGMENT FAULT\n{:#?}\nERROR CODE : {:#?}", stack_frame, SelectorErrorCode::new(error_code));
}

extern "x86-interrupt" fn invalid_tss_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    panic!("EXCEPTION: INVALID TSS\n{:#?}\nERROR CODE : {:#?}", stack_frame, SelectorErrorCode::new(error_code));
}

extern "x86-interrupt" fn invalid_op_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: INVALID OPCODE\n{:#?}\n", stack_frame);
}

extern "x86-interrupt" fn alignment_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    panic!("EXCEPTION: OUT OF ALIGNMENT\n{:#?}\nERROR CODE : {:#?}", stack_frame, SelectorErrorCode::new(error_code));
}

extern "x86-interrupt" fn segment_not_present_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    panic!("EXCEPTION: SEGMENT NOT PRESENT\n{:#?}\nERROR CODE : {:#?}", stack_frame, SelectorErrorCode::new(error_code));
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


///Doesnt do anything at the moment
///TODO: Notify the ata caller that the ata controller is ready
extern "x86-interrupt" fn ata_interrupt_handler(_stack_frame: InterruptStackFrame) {
    unsafe {
        PICS.lock().notify_end_of_interrupt(InterruptIndex::PrimATA.as_u8());
    }
}


///Reads the key code from 0x60 port and adds that to the keyboard task handler
extern "x86-interrupt" fn mouse_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;
    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe{ port.read() };
    unsafe { mouse::add_scancode(scancode) };
    

    unsafe {
        PICS.lock().notify_end_of_interrupt(InterruptIndex::Mouse.as_u8());
    }
}

///Reads the key code from 0x60 port and adds that to the keyboard task handler
extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;
    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe{ port.read() };
    keyboard::add_scancode(scancode); 

    unsafe {
        PICS.lock().notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

use x86_64::structures::idt::PageFaultErrorCode;
use x86_64::registers::control::Cr2;

///Page fault handler prints out the respective errors and stack frame and halts cpu execution
extern "x86-interrupt" fn page_fault_handler(_stack_frame: InterruptStackFrame, _error_code: PageFaultErrorCode) {
    loop {
        x86_64::instructions::hlt();
    }
    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("Error Code: {:?}", _error_code);
    println!("{:#?}", _stack_frame);

    // unsafe {
    //     PICS.lock().notify_end_of_interrupt(0xE);
    // }
   
}


///Used for task time slices
extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    
    unsafe {
        PICS.lock().notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

///Double fault interrupt panics and prints the stack frame
extern "x86-interrupt" fn double_fault_handler(stack_frame: InterruptStackFrame, _error_code: u64) -> ! {
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

