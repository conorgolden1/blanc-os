use core::slice;

use printer::{print, println};



global_asm!(include_str!("syscall_interrupts.s"));

extern "C" {
    pub fn syscall(call_num: u64, param1: u64, param2: u64, param3: u64) -> u64;
}

pub(crate) static SYSTEM_CALLS: [fn(param1: u64, param2: u64, param3: u64); 2] = [
    // Syscall 0
    print, // Syscall 1
    exit,
];

fn print(_file_descriptor: u64, affective_address: u64, bytes: u64) {
    // TODO IMPLEMENT FD AND ERROR CHECKING
    unsafe {
        let slice = slice::from_raw_parts(affective_address as *const _, bytes as usize);
        match core::str::from_utf8(slice) {
            Ok(str) => print!("{}", str),
            Err(_) => {
                let number = affective_address as *const u64;

                print!("{}", *number)
            }
        }
    }
}


/// Get the current running task from the mutex scheduler,
/// changed the tasks state to finished then use a timer interrupt
/// to invoke the scheduler, the scheduler will then take the current
/// task off of a ready queue so it won't execute again
fn exit(_: u64, _: u64, _: u64) {
    use task::scheduler::Scheduler;
    use task::task::{TaskState, Task};
    use crate::InterruptIndex;

    let mut scheduler = Scheduler::get_scheduler();
    let running_task: &mut Task = scheduler
        .running_task()
        .unwrap_or_else(|| unreachable!());
    
    running_task.set_state(TaskState::Finished);

    unsafe {
        asm!(
            "int 32"
        );
    }
    // We somehow need to get the current task and change its state to finished then invoke the scheduler
    // Should we store the current task as a mutable global? maybe
    // then invoke the scheduler from here
}
