use x86_64::{
    structures::paging::{PageTable, PhysFrame},
    VirtAddr,
};

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
pub unsafe fn new_context_switch(
    page_table: PhysFrame,
    stack_top: VirtAddr,
    entry_point: VirtAddr,
) -> ! {
    unsafe {
        asm!(
            "mov cr3, {}; mov rsp, {}; push 0; jmp {}",
            in(reg) page_table.start_address().as_u64(),
            in(reg) stack_top.as_u64(),
            in(reg) entry_point.as_u64(),
        );
    }
    unreachable!();
}
