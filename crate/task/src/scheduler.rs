use crossbeam_queue::{ArrayQueue, PushError};
use printer::{print, println};
use spin::{Mutex, MutexGuard, Once};

use crate::task::Task;

/// Only 1000 processes are allowed in the ready queue
static PROCESS_CAPACITY: usize = 1000;
/// Only 100 processes are allowed in the new queue
static NEW_CAPACITY: usize = 100;

/// Global scheduler so we can invoke it from an exit syscall or from
/// the timer interrupt
pub static SCHEDULER: Once<Mutex<Scheduler>> = Once::new();

/// Scheduler for handling tasks when a given tasks time slice is up
/// The scheduler should be called from the timer interrupt and change the
/// task that we return too on that interrupt
pub struct Scheduler {
    ready_queue: ArrayQueue<Task>,
    new_queue: ArrayQueue<Task>,
    running_task : Option<Task>,
}

impl Scheduler {
    pub fn init() {
        SCHEDULER.call_once(|| {
            Mutex::new(Self {
                ready_queue: ArrayQueue::<Task>::new(PROCESS_CAPACITY),
                new_queue: ArrayQueue::<Task>::new(NEW_CAPACITY),
                running_task : None,
            })
        });
    }

    /// Add a new task to the new queue so the scheduler will enter it given
    /// the next schedulers time slice
    pub fn add_task(task: Task) -> Result<(), PushError<Task>> {
        let scheduler = Scheduler::get_scheduler();
        scheduler.new_queue.push(task)
    }

    /// Transfer control from the kernels main to the scheduler
    /// this will only be called once because it will handle control to the first
    /// task in the created queue and then the scheduler will only be invoked from
    /// the timer interrupt from here on out
    pub fn run() {
        use memory::swap_to_kernel_table;
        use crate::task::TaskState;
        use core::mem::replace;

        unsafe {
            asm!("
                push rbp
                mov  rbp, rsp
                push r15
                push r14
                push r13
                push r12
                push r11
                push r10
                push r9
                push r8
                push rdi
                push rsi
                push rdx
                push rcx
                push rbx
                push rax
            ");
        }

        let mut scheduler = Scheduler::get_scheduler();
        
        //TODO: ISOLATE STATE CHANGE SOMEHOW

        if !scheduler.new_queue.is_empty() {
        // Might be a problem since were not returning interrupt
        // Also were not saving an old tasks state, consider using a get context method
            swap_to_kernel_table();
            
            let task = scheduler.new_queue.pop().unwrap();

            //////////////////////// TO ISOLATE //////////////////////////
            let running_task = scheduler.running_task();

            // Move the old task to the ready queue
            if let Some(r_task) = running_task {
                match r_task.state() {
                    TaskState::Running => r_task.set_state(TaskState::Ready),
                    TaskState::Blocked => todo!("implement a blocked queue for io req"),
                    TaskState::Finished => todo!("implement a finished list for ended tasks"),
                    _ => ()
                }
                let old_task = replace::<Task>(r_task, task);
                scheduler.ready_queue.push(old_task).expect("Ready queue is full");
            } else {
                scheduler.set_running_task(Some(task));
            }
            //^^^^^^^^^^^^^^^^^^^^^ TO ISOLATE ^^^^^^^^^^^^^^^^^^^^^^^^^//

            let task = scheduler.running_task().unwrap();

            println!("New Swapping to {}", task.name);
            task.set_state(TaskState::Running);
            
            Scheduler::swap_to_task_table(task);
            
            unsafe {
                x86_64::registers::control::Efer::write_raw(
                    x86_64::registers::control::Efer::read_raw() ^ 2 ^ 11,
                );
                
                asm!(
                    "jmp {}",
                    in(reg) task.entry_point()
                );
            }
        } 
        if !scheduler.ready_queue.is_empty() {
            // First prepare old task
            // Then get ready to execute new task

            let task = scheduler.ready_queue.pop().unwrap();

            //////////////////////// TO ISOLATE //////////////////////////
            let running_task = scheduler.running_task();

            // Move the old task to the ready queue
            if let Some(r_task) = running_task {
                match r_task.state() {
                    TaskState::Running => r_task.set_state(TaskState::Ready),
                    TaskState::Blocked => todo!("implement a blocked queue for io req"),
                    TaskState::Finished => todo!("implement a finished list for ended tasks"),
                    _ => ()
                }
                let old_task = replace::<Task>(r_task, task);
                scheduler.ready_queue.push(old_task).expect("Ready queue is full");
            } else {
                scheduler.set_running_task(Some(task));
            }
            //^^^^^^^^^^^^^^^^^^^^^ TO ISOLATE ^^^^^^^^^^^^^^^^^^^^^^^^^//

            let task = scheduler.running_task().unwrap();

            println!("Ready Swapping to {}", task.name);
            task.set_state(TaskState::Running);

            Scheduler::swap_to_task_table(task);

            unsafe {
                asm!(
                    "
                    mov  rsp, rax
                    pop rax
                    pop rbx
                    pop rcx
                    pop rdx
                    pop rsi
                    pop rdi
                    pop r8
                    pop r9
                    pop r10
                    pop r11
                    pop r12
                    pop r13
                    pop r14
                    pop r15
                    pop rbp
                    ",
                );
            }
            
        }

    }

    /// Get the current scheduler from the static lock
    pub fn get_scheduler() -> MutexGuard<'static, Scheduler> {
        SCHEDULER.wait().expect("Scheduler unitialized").lock()
    }

    /// Get a reference to the scheduler's running task.
    pub fn running_task(&mut self) -> Option<&mut Task> {
        self.running_task.as_mut()
    }

    fn swap_to_task_table(task : &Task) {
        use memory::RECURSIVE_INDEX;
        use core::ops::Index;
        use x86_64::registers::control::Cr3;
        use x86_64::structures::paging::PhysFrame;
        let page_table = task.page_table();

        unsafe {
            Cr3::write(
                PhysFrame::containing_address(page_table.index(511).addr()),
                Cr3::read().1,
            )
        };
        *RECURSIVE_INDEX.wait().unwrap().lock() = 508;
        x86_64::instructions::tlb::flush_all();
    }

    /// Set the scheduler's running task.
    pub fn set_running_task(&mut self, running_task: Option<Task>) {
        self.running_task = running_task;
    }
}
