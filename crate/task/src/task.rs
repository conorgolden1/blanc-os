use core::{
    ops::Index,
    sync::atomic::{AtomicUsize, Ordering},
};

use elfloader::ElfBinary;
use memory::{
    kpbox::KpBox, swap_to_kernel_table, KERNEL_PAGE_TABLE, RECURSIVE_INDEX,
};

use x86_64::{
    registers::control::Cr3,
    structures::paging::{
        PageTable, PageTableFlags, PhysFrame,
    },
    VirtAddr,
};


use crate::elf::ElfMemory;

extern crate alloc;
pub struct Task {
    task_id: TaskID,
    pub entry: VirtAddr,
    page_table : KpBox<PageTable>,
    state: TaskState,
    pub name: &'static str,
    pub ring: Ring,
}

impl Task {

    ///
    ///
    /// name   : Name of the executable (TODO consider making this an option)
    /// bin    : A slice of bytes containing the executable data
    /// ring   : Ring that this executable will be in (TODO consider making default ring 3)
    /// offset : Offset in virtual memory that the program will be loaded too (TODO consider making a default offset)
    pub fn binary(
        name: Option<&'static str>,
        bin: &[u8],
        ring: Option<Ring>,
        offset: Option<u64>,
    ) -> Task {
        let offset = offset.unwrap_or(0x81_FF00_0000);

        let ring = ring.unwrap_or(Ring::Ring3);

        let name = name.unwrap_or("");

        let page_table = Pml4Creator::default().create();

        unsafe {
            Cr3::write(
                PhysFrame::containing_address(page_table.index(511).addr()),
                Cr3::read().1,
            )
        };
        *RECURSIVE_INDEX.wait().unwrap().lock() = 511;
        x86_64::instructions::tlb::flush_all();

        let elf = ElfBinary::new(bin).unwrap();
        let mut loader = ElfMemory::new(offset);
        elf.load(&mut loader).unwrap();

        let entry = VirtAddr::new(elf.entry_point() + offset);

        
        swap_to_kernel_table();

        Self {
            task_id: TaskID::allocate(),
            entry,
            state: TaskState::New,
            ring,
            name,
            page_table,
        }
    }

    pub fn task_id(&self) -> TaskID {
        self.task_id
    }

    pub fn state(&self) -> TaskState {
        self.state
    }

    pub fn entry_point(&self) -> u64 {
        self.entry.as_u64()
    }

    // pub fn stack_frame_top_addr(&self) -> VirtAddr {
    //     self.stack_frame.virt_addr()
    // }

    // pub fn stack_frame_bottom_addr(&self) -> VirtAddr {
    //     let b = self.stack_frame.bytes();
    //     self.stack_frame_top_addr() + b.as_usize()
    // }

    /// Set the task's state.
    pub fn set_state(&mut self, state: TaskState) {
        self.state = state;
    }

    /// Get a reference to the task's page table.
    pub fn page_table(&self) -> &KpBox<PageTable> {
        &self.page_table
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
    /// Process has just been created and needs to be jumped
    /// to from its entry point not a context switch
    New,

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

#[derive(Default)]
pub struct Pml4Creator {
    pml4: KpBox<PageTable>,
}
impl Pml4Creator {
    pub fn create(mut self) -> KpBox<PageTable> {
        self.map_kernel_area();
        self.enable_recursive_paging();
        self.pml4
    }

    fn enable_recursive_paging(&mut self) {
        let a = PhysFrame::containing_address(self.pml4.phys_addr());
        let f =
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE;
        self.pml4[511].set_frame(a, f);
    }

    fn map_kernel_area(&mut self) {
        self.pml4[0] = KERNEL_PAGE_TABLE.wait().unwrap().lock().level_4_table()[0].clone();
        self.pml4[256] = KERNEL_PAGE_TABLE.wait().unwrap().lock().level_4_table()[256].clone();
        for i in 507..512 {
            self.pml4[i] = KERNEL_PAGE_TABLE.wait().unwrap().lock().level_4_table()[i].clone();
        }
    }
}
