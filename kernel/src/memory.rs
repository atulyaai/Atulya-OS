use x86_64::{
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PageTableFlags,
        PhysFrame, Size4KiB, PageSize,
    },
    registers::control::Cr3,
    PhysAddr, VirtAddr,
};

pub struct BootInfoFrameAllocator {
    next_free: PhysAddr,
}

impl BootInfoFrameAllocator {
    pub unsafe fn init() -> Self {
        Self {
            next_free: PhysAddr::new(0x1000),
        }
    }

    pub unsafe fn with_start(start: PhysAddr) -> Self {
        Self {
            next_free: start,
        }
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = PhysFrame::containing_address(self.next_free);
        self.next_free += Size4KiB::SIZE;
        Some(frame)
    }
}

pub struct MemoryMapper {
    page_table: OffsetPageTable<'static>,
}

impl MemoryMapper {
    pub unsafe fn new(physical_memory_offset: VirtAddr) -> Self {
        let (level_4_page_table_frame, _) = Cr3::read();
        let page_table_ptr = level_4_page_table_frame.start_address().as_u64() as *mut PageTable;
        let page_table = &mut *page_table_ptr;
        let page_table = OffsetPageTable::new(page_table, physical_memory_offset);
        Self { page_table }
    }

    pub unsafe fn map_to<A>(
        &mut self,
        page: Page<Size4KiB>,
        frame: PhysFrame<Size4KiB>,
        flags: x86_64::structures::paging::PageTableFlags,
        allocator: &mut A,
    ) -> Result<x86_64::structures::paging::mapper::MapperFlush<Size4KiB>, x86_64::structures::paging::mapper::MapToError<Size4KiB>>
    where
        A: FrameAllocator<Size4KiB>,
    {
        self.page_table
            .map_to(page, frame, flags, allocator)
    }
}

/// Map physical memory for the heap into the kernel's page table.
/// Uses identity mapping (physical == virtual) since BIOS bootloader uses phys_offset=0.
pub fn init_heap(heap_phys_start: usize, heap_size: usize) {
    let page_start = PhysAddr::new(heap_phys_start as u64);
    let page_end = PhysAddr::new((heap_phys_start + heap_size - 1) as u64);
    let start_frame = PhysFrame::<Size4KiB>::containing_address(page_start);
    let end_frame = PhysFrame::<Size4KiB>::containing_address(page_end);

    // Start frame allocator after the heap to avoid overwriting bootloader/kernel data
    let alloc_start = PhysAddr::new((heap_phys_start + heap_size) as u64);

    unsafe {
        let mut mapper = MemoryMapper::new(VirtAddr::new(0));
        let mut frame_allocator = BootInfoFrameAllocator::with_start(alloc_start);

        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

        let mut frame = start_frame;
        while frame <= end_frame {
            let page = Page::<Size4KiB>::containing_address(
                VirtAddr::new(frame.start_address().as_u64()),
            );
            // Ignore if already mapped (e.g., bootloader mapped it)
            let _ = mapper.map_to(page, frame, flags, &mut frame_allocator);
            frame = PhysFrame::containing_address(frame.start_address() + Size4KiB::SIZE);
        }
    }
}
