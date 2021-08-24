//! This module is used for asynchronous executor to run asynchronous OS tasks
use super::{TaskId, Task};
use alloc::{collections::BTreeMap, sync::Arc, task::Wake};
use core::task::{Context, Poll, Waker};
use crossbeam_queue::ArrayQueue;

/// The Executor that will hold tasks, task_queue, and the waker cache
pub struct Executor {
    tasks: BTreeMap<TaskId, Task>,
    task_queue: Arc<ArrayQueue<TaskId>>,
    waker_cache: BTreeMap<TaskId, Waker>,
}

impl Executor {
    /// Create a new executor type to hold tasks, task_queue, and the waker cache
    pub fn new() -> Self {
        Executor {
            tasks: BTreeMap::new(),
            task_queue: Arc::new(ArrayQueue::new(100)),
            waker_cache: BTreeMap::new(),
        }
    }
    /// Spawn a new task in the Executor's queue
    pub fn spawn(&mut self, task: Task) {
        let task_id = task.id;
        if self.tasks.insert(task.id, task).is_some() {
            panic!("task with same ID already in tasks");
        }
        self.task_queue.push(task_id).expect("Async task queue full");
    }

    /// Run any tasks that are in the queue
    fn run_ready_tasks(&mut self) {
        let Self { 
            tasks,
            task_queue,
            waker_cache } = self;
        
        while let Ok(task_id) = task_queue.pop() {
            let task = match tasks.get_mut(&task_id) {
                Some(task) => task,
                None => continue,
            };
            let waker = waker_cache.entry(task_id).or_insert_with(
                || TaskWaker::new(task_id, task_queue.clone())
            );

            let mut context = Context::from_waker(waker);
            match task.poll(&mut context) {
                Poll::Ready(()) => {
                    tasks.remove(&task_id);
                    waker_cache.remove(&task_id);
                }
                Poll::Pending => {}
            }
        }
    }

    /// Run any ready tasks then check if there are tasks in the queue and sleep
    pub fn run(&mut self) -> ! {
        loop {
            self.run_ready_tasks();
            self.sleep_if_idle();
        }
    }

    /// Sleep if there are no tasks in the task queue
    fn sleep_if_idle(&self) {
        use x86_64::instructions::interrupts::{self, enable_and_hlt};

        interrupts::disable();
        if self.task_queue.is_empty() {
            enable_and_hlt();
        } else {
            interrupts::enable();
        }
    }
}


/// A waker struct that holds the task ID and a reference of the task queue 
pub struct TaskWaker {
    task_id: TaskId,
    task_queue: Arc<ArrayQueue<TaskId>>,
}

impl TaskWaker {
    /// Converts a task_id and a reference to the task queue from a task waker to a Waker
    pub fn new(task_id: TaskId, task_queue: Arc<ArrayQueue<TaskId>>) -> Waker {
        Waker::from(Arc::new(TaskWaker {
            task_id,
            task_queue,
        }))
    }

    /// Push a task id onto the task queue for the executor to 'execute'
    fn wake_task(&self) {
        self.task_queue.push(self.task_id).expect("task_queue full");
    }
}


impl Wake for TaskWaker {
    /// Wrapper implementation for Wake functionality with the TaskWaker struct
    fn wake(self: Arc<Self>) {
        self.wake_task();
    }
    /// Wrapper implementation for Wake functionality with the TaskWaker struct by reference
    fn wake_by_ref(self: &Arc<Self>) {
        self.wake_task();
    }
}
