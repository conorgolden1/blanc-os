/// Context of registers used for task switching
#[derive(Default)]
#[repr(C, packed)]
pub struct Context {
    cr3: u64,
    rbp: u64,
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,
    rbx: u64,
    rflags: u64,
    rip: u64,
}



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
