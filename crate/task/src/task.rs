use core::{convert::TryInto, ops::Index, sync::atomic::{AtomicUsize, Ordering}};


use crate::{elf::Pml4Creator, stack_frame::StackFrame};
use memory::{KERNEL_PAGE_TABLE, RECURSIVE_INDEX, active_level_4_table, kpbox::KpBox, swap_to_kernel_table};
use printer::{println, print};
use x86_64::{VirtAddr, registers::control::Cr3, structures::paging::{PageSize, PageTable, PageTableIndex, PhysFrame, RecursivePageTable, Size4KiB}};

extern crate alloc;
pub struct Task {
    task_id: TaskID,
    pub entry: VirtAddr,
    pub pml4: KpBox<PageTable>,
    state: TaskState,
    stack: KpBox<[u8]>,
    stack_frame: KpBox<StackFrame>,
    pub name: &'static str,
    pub ring: Ring,
}

impl Task {
    const STACK_SIZE: u64 = Size4KiB::SIZE;

    pub fn binary(name: &'static str, bin: &[u8], ring: Ring) -> Task {
        
        let mut page_table = Pml4Creator::default().create();
        println!("Writing {} Page_Table", name);
        println!("BEFORE CR3 Write {:#?}", Cr3::read().0);
        unsafe {Cr3::write(PhysFrame::containing_address(page_table.index(511).addr()), Cr3::read().1)};
        println!("AFTER CR3 Write {:#?}", Cr3::read().0);
        *RECURSIVE_INDEX.wait().unwrap().lock() = 511;
        x86_64::instructions::tlb::flush_all();
        
        
        let mut pml4  = RecursivePageTable::new(active_level_4_table()).unwrap();
    
        let mut shell_proc = crate::elf::Loader::new(bin, &mut pml4).unwrap();
        shell_proc.load_segments().unwrap();
        
        let entry = shell_proc.entry_point();
        let stack = KpBox::new_slice(0, Self::STACK_SIZE.try_into().unwrap());
        let stack_bottom = stack.virt_addr() + stack.bytes().as_usize();
        let stack_frame = KpBox::from(match ring {
            Ring::Ring0 => StackFrame::kernel(entry, stack_bottom),
            Ring::Ring3 => StackFrame::user(entry, stack_bottom),
        });
        
        println!("Mapping {} stack", name);
        shell_proc.map_page_box(&stack);
        shell_proc.map_page_box(&stack_frame);

        swap_to_kernel_table();

        Self {
            task_id: TaskID::allocate(),
            entry,
            state: TaskState::Ready,
            stack_frame,
            stack,
            ring,
            name,
            pml4 : page_table,
        }
    }

    pub fn task_id(&self) -> TaskID {
        self.task_id
    }

    pub fn state(&self) -> TaskState {
        self.state
    }

    pub fn stack_frame_top_addr(&self) -> VirtAddr {
        self.stack_frame.virt_addr()
    }

    pub fn stack_frame_bottom_addr(&self) -> VirtAddr {
        let b = self.stack_frame.bytes();
        self.stack_frame_top_addr() + b.as_usize()
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
