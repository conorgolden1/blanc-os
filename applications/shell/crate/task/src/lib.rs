#![no_std]
#![feature(asm)]
#![feature(naked_functions)]
#![feature(ptr_internals)]
#![feature(thread_local)]

use scheduler::add;
use task::{Ring, Task};

pub mod context_switch;
pub mod elf;
pub mod scheduler;
pub mod stack_frame;
pub mod task;

pub fn binary(name: &'static str, bin: &[u8], ring: Ring) -> Task {
    let proc = task::Task::binary(name, bin, ring);
    //push_process_to_queue(proc);
    proc
}

fn push_process_to_queue(task: Task) {
    add(task.task_id());
}
