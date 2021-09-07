use bootloader::boot_info::TlsTemplate;
use memory::{kpbox::KpBox, phys::FRAME_ALLOCATOR, KERNEL_PAGE_TABLE};
use printer::{print, println};
use x86_64::{
    structures::paging::{
        mapper::MapperAllSizes, FrameAllocator, Page, PageSize, PageTable, PageTableFlags,
        PhysFrame, Size4KiB,
    },
    VirtAddr,
};
use xmas_elf::{
    header,
    program::{self},
    ElfFile,
};
pub struct Loader<'a, M> {
    elf_file: ElfFile<'a>,
    inner: Inner<'a, M>,
}

impl<'a, M> Loader<'a, M>
where
    M: MapperAllSizes,
{
    pub fn new(bytes: &'a [u8], page_table: &'a mut M) -> Result<Self, &'static str> {
        println!("Elf file loaded at {:#p}", bytes);

        let elf_file = ElfFile::new(bytes)?;
        header::sanity_check(&elf_file)?;

        let loader = Loader {
            elf_file,
            inner: Inner { page_table },
        };

        Ok(loader)
    }

    pub fn load_segments(&mut self) -> Result<Option<TlsTemplate>, &'static str> {
        let mut _tls_template = None;
        for program_header in self.elf_file.program_iter() {
            program::sanity_check(program_header, &self.elf_file)?;

            match program_header.get_type()? {
                program::Type::Load => self.inner.handle_load_segment(program_header)?,
                program::Type::Tls => todo!(),
                program::Type::Null
                | program::Type::Dynamic
                | program::Type::Interp
                | program::Type::Note
                | program::Type::ShLib
                | program::Type::Phdr
                | program::Type::GnuRelro
                | program::Type::OsSpecific(_)
                | program::Type::ProcessorSpecific(_) => {}
            }
        }

        Ok(_tls_template)
    }

    pub(super) fn map_page_box(&mut self, b: &KpBox<impl ?Sized>) {
        use core::convert::TryFrom;
        for i in 0..b.bytes().as_num_of_pages::<Size4KiB>().as_usize() {
            let off = Size4KiB::SIZE * u64::try_from(i).unwrap();
            let page = Page::from_start_address(b.virt_addr() + off).expect("Page is not aligned.");
            let frame = FRAME_ALLOCATOR.wait().unwrap().allocate_frame().unwrap();
            self.map(page, frame);
        }
    }

    fn map(&mut self, page: Page<Size4KiB>, frame: PhysFrame<Size4KiB>) {
        let flags =
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE;
        unsafe {
            self.inner
                .page_table
                .map_to(page, frame, flags, FRAME_ALLOCATOR.wait().as_mut().unwrap())
                .unwrap()
                .flush()
        };
    }

    pub fn get_table(&'a mut self) -> &'a mut M {
        self.inner.page_table
    }

    pub fn entry_point(&self) -> VirtAddr {
        VirtAddr::new(self.elf_file.header.pt2.entry_point())
    }
}

struct Inner<'a, M> {
    page_table: &'a mut M,
}

impl<'a, M> Inner<'a, M>
where
    M: MapperAllSizes,
{
    pub(crate) fn handle_load_segment(
        &mut self,
        segment: program::ProgramHeader,
    ) -> Result<(), &'static str> {
        println!("Handling Load Segment, {:x?}", segment);

        let virt_start_addr = VirtAddr::new(segment.offset() + segment.virtual_addr());
        let virt_end_addr = virt_start_addr + segment.mem_size();
        let virt_start_page = Page::<Size4KiB>::containing_address(virt_start_addr);
        let virt_end_page = Page::<Size4KiB>::containing_address(virt_end_addr);
        let page_range = Page::range_inclusive(virt_start_page, virt_end_page);

        let mut segment_flags = PageTableFlags::PRESENT;
        if !segment.flags().is_execute() {
            segment_flags |= PageTableFlags::NO_EXECUTE;
        }
        if segment.flags().is_write() {
            segment_flags |= PageTableFlags::WRITABLE;
        }

        for page in page_range {
            let frame = FRAME_ALLOCATOR
                .wait()
                .unwrap()
                .allocate_frame()
                .ok_or("Frame Allocation Error")?;

            let flusher = unsafe {
                self.page_table
                    .map_to(
                        page,
                        frame,
                        segment_flags,
                        FRAME_ALLOCATOR.wait().as_mut().unwrap(),
                    )
                    .map_err(|_err| "map_to failed")?
            };
            flusher.ignore();
        }

        // // Handle .bss section (mem_size > file_size)
        // if segment.mem_size() > segment.file_size() {
        //     // .bss section (or similar), which needs to be mapped and zeroed
        //     self.handle_bss_section(&segment, segment_flags)?;
        // }

        Ok(())
    }
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
        println!("NEW PT : {:#?}", a);
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
