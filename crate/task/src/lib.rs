#![no_std]
#![feature(asm)]
#![feature(naked_functions)]
#![feature(ptr_internals)]
#![feature(thread_local)]

pub mod scheduler;
pub mod context_switch;

use core::sync::atomic::{AtomicUsize, Ordering};

extern crate alloc;
pub struct Task {
    task_id : TaskID,
    state : TaskState,
    context : Context,
}

impl Task {
    fn task_id(&self) -> TaskID {
        self.task_id
    }

    pub fn state(&self) -> TaskState {
        self.state
    }
}



/// Ring enum representing what ring the task is for
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Ring {
    Ring0 = 0b00,
    Ring3 = 0b11,
}



#[derive(Clone, Copy, PartialEq, Eq, Hash)]
/// TaskID struct used for atomically getting new task ID's
pub struct TaskID(usize);

impl TaskID {
    /// Create a new task ID with a given Process ID
    pub const fn new(pid: usize) -> TaskID {
        TaskID(pid)
        
    }

    /// Allocate a new Task ID with an atomically incrementing process id
    fn allocate() -> TaskID {
        static _NEXT_PID: AtomicUsize = AtomicUsize::new(1);

        Self::new(_NEXT_PID.fetch_add(1, Ordering::AcqRel))
    }

    /// Get the task id
    pub fn get_id(&self) -> usize {
        self.0
    }
}


/// An enum describing the state of a task
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TaskState {
    /// Process is ready for execution
    Ready,

    /// Process is currently running
    Running,

    /// Process is blocked from IO
    Blocked,

    /// Process has finished execution
    Finished,
}


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
