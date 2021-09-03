//! Global Descriptor Table functionalitity
#![no_std]

use x86_64::structures::gdt::GlobalDescriptorTable;
use x86_64::structures::tss::TaskStateSegment;
use lazy_static::lazy_static;

/// Index in the interrupt stack table in the TSS for a double fault
pub const DOUBLE_FAULT_INDEX: u16 = 0;

lazy_static! {
    /// Global Static Reference to the kernels task state segment
    ///
    /// This task state segment is initialized with a stack to handle
    /// the double fault interrupt and is the only entry in the TSS
    /// interrupt stack table. 
    static ref TSS: TaskStateSegment = {
        use x86_64::VirtAddr;

        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_INDEX as usize] = {
            // Stack size = 20kb
            const STACK_SIZE: usize = 4096 * 5;
            // Zero out a array equal to the stack size
            static  mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
            
            let stack_start = VirtAddr::from_ptr(unsafe { &STACK });
            
            // Assign the table entry to point to the end of the stack
            stack_start + STACK_SIZE
        };
        tss
    };

   
    /// Global Static Reference to the kernels Global Descriptor Table
    ///
    /// This global descriptor table is assigned 
    pub static ref GDT: (GlobalDescriptorTable, Selectors) = {
        use x86_64::structures::gdt::Descriptor;

        let mut gdt = GlobalDescriptorTable::new();
        let kernel_code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let kernel_data_selector = gdt.add_entry(Descriptor::kernel_data_segment());
        let user_code_selector = gdt.add_entry(Descriptor::user_code_segment());
        let user_data_selector = gdt.add_entry(Descriptor::user_data_segment());
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
        
        (gdt, Selectors { kernel_data_selector, kernel_code_selector, tss_selector, user_code_selector, user_data_selector })
    };
}



/// Load the global desciptor table into memory and set the code and tss selectors
pub fn init() {
    use x86_64::instructions::segmentation::{CS, DS, Segment};
    use x86_64::instructions::tables::load_tss;

    // Load the global descriptor table into memory
    GDT.0.load();

    unsafe {
        //Set the code register with the code selector
        CS::set_reg(GDT.1.kernel_code_selector);
        DS::set_reg(GDT.1.kernel_data_selector);
        //Load the TSS selector
        load_tss(GDT.1.tss_selector);
    }
}

use x86_64::structures::gdt::SegmentSelector;
/// Selectors for the two GDT entries, the kernel code segment,
/// and the task state segment
pub struct Selectors {
    pub kernel_data_selector: SegmentSelector,
    pub kernel_code_selector: SegmentSelector,
    pub user_code_selector : SegmentSelector,
    pub user_data_selector : SegmentSelector,
    tss_selector: SegmentSelector,
}
