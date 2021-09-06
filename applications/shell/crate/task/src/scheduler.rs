use alloc::collections::VecDeque;
use spin::{Mutex, MutexGuard, Once};

use crate::task::TaskID;

extern crate alloc;

static PID_QUEUE: Once<Mutex<VecDeque<TaskID>>> = Once::new();

pub fn add(id: TaskID) {
    lock_queue().push_back(id)
}

pub fn change_active_pid() {
    lock_queue().rotate_left(1)
}

pub fn active_pid() -> TaskID {
    lock_queue()[0]
}

pub fn pop() -> TaskID {
    lock_queue().pop_front().expect("Empty Process Queue")
}

fn lock_queue() -> MutexGuard<'static, alloc::collections::VecDeque<TaskID>> {
    PID_QUEUE.wait().unwrap().lock()
}
