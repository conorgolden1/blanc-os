#![no_std]
#![feature(asm)]
#![feature(naked_functions)]
#![feature(ptr_internals)]

use core::{ptr::Unique, sync::atomic::{AtomicUsize, Ordering}, task::Context};

pub mod context_switch;

pub struct Task {
    // context : Unique<Context>,

}




/// Ring enum representing what ring the task is for
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Ring {
    Ring0 = 0b00,
    Ring3 = 0b11,
}




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