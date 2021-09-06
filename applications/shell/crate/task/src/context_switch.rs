use core::ops::Index;

use gdt::TSS;
use x86_64::{VirtAddr, registers::control::Cr3, structures::paging::{PageTable, PhysFrame}};

use crate::task::Task;

/// Switch the context between two tasks
///
///
///
/// # Safety
/// The user must disable interrupts before switching.
/// Switching stack pointers to invalid places can cause undefined behavior
#[naked]
pub unsafe extern "C" fn context_switch(_prev_sp: *mut usize, _next_sp: usize) {
    asm!(
        //Save general purpose registers
        r#"
            push rbx
            push rbp
            push r12
            push r13
            push r14
            push r15
        "#,
        // Save CR3 register
        r#"
            mov rax, cr3
            push rax  
        "#,
        //Switch tasks
        r#"
            mov [rdi], rsp
            mov rsp, rsi
        "#,
        // Restore CR3 register
        r#"
            pop rax
            mov rax, cr3 
        "#,
        //Restore the next task's general purpose registers
        r#" 

            pop r15
            pop r14
            pop r13
            pop r12
            pop rbp
            pop rbx
            ret
        "#,
        options(noreturn)
    );
}

/// Performs the actual context switch.
/// # Safety
///  TODO
pub unsafe fn new_context_switch(
    mut process : Task
) -> ! {
    switch_pml4(&mut process);
    // TSS.lock().interrupt_stack_table[0] = process.stack_frame_bottom_addr();
    asm!(
        "mov rsp, {}; push 0; jmp {}",
        in(reg) process.stack_frame_top_addr().as_u64(),
        in(reg) process.entry.as_u64(),
    );
    unreachable!()
}


fn switch_pml4(process : &mut Task) {
    let (_, f) = Cr3::read();
    let pml4 = &mut process.pml4;
 
    // SAFETY: The PML4 frame is correct one and flags are unchanged.
    unsafe { Cr3::write(PhysFrame::from_start_address(pml4.phys_addr()).unwrap(), f) }
}