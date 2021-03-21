use x86_64::{
    structures::paging::PageTable,
    structures::paging::OffsetPageTable,
    VirtAddr,
     structures::paging::{Page, PhysFrame, Mapper, Size4KiB, FrameAllocator},
    PhysAddr,
};

use bootloader::bootinfo::{MemoryMap, MemoryRegionType};

/// Initialize a new OffsetPageTable.
///
/// # Safety
/// 
/// This function is unsafe as the caller must guarantee that the entirety
/// of physical memory is mapped to virtual memory at the provided `physical_memory_offset`;
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let l4_table = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(l4_table, physical_memory_offset)
}

/// Returns a mutable reference to the active level 4 page table.
///
/// # Safety
/// 
/// This function is unsafe as the caller must guarantee that the
/// complete pyhsical memory is mapped to virtual memory at the
/// provided `phsical_memory_offset`.
/// It must also be called only once to avoid aliasing &mut references
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;
    let (l4_frame, _) = Cr3::read();
    let phys = l4_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();
    &mut *page_table_ptr
}

pub fn create_example_mapping(page: Page, mapper: &mut OffsetPageTable, frame_allocator: &mut impl FrameAllocator<Size4KiB>) {
    use x86_64::structures::paging::PageTableFlags as Flags;
    let frame = PhysFrame::containing_address(PhysAddr::new(0xb8000));
    let flags = Flags::PRESENT | Flags::WRITABLE;
    let map_to_result = unsafe {
        // FIXME this is an example that should not stick around
        mapper.map_to(page, frame, flags, frame_allocator)
    };
    map_to_result.expect("map_to failed").flush();
}

pub struct EmptyFrameAllocator;

unsafe impl FrameAllocator<Size4KiB> for EmptyFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        None
    }
}

pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryMap,
    next: usize,
}
impl BootInfoFrameAllocator {
    /// Create a `FrameAllocator` from the passed memory map.
    ///
    /// # Safety
    /// 
    /// The caller must guarantee that the passed memory map is valid.
    /// The main requirement is that all frames marked as `USABLE` are
    /// really unused.
    pub unsafe fn init(memory_map: &'static MemoryMap) -> Self {
        BootInfoFrameAllocator {
            memory_map,
            next: 0
        }
    }

    /// Returns an iterator over usable frames specified by the memory map.
    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        let regions = self.memory_map.iter();
        let usable_regions = regions
            .filter(|r| r.region_type == MemoryRegionType::Usable);
        let addr_ranges = usable_regions.map(|r| r.range.start_addr()..r.range.end_addr());
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}
