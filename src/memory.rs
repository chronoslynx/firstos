use x86_64::{
    structures::paging::OffsetPageTable,
    structures::paging::PageTable,
    structures::paging::{FrameAllocator, FrameDeallocator, PhysFrame, Size4KiB},
    PhysAddr, VirtAddr,
};

use bootloader::bootinfo::{BootInfo, MemoryRegionType};

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

struct FrameInfo {
    next: Option<&'static mut FrameInfo>,
}

impl FrameInfo {
    fn start_addr(&self) -> usize {
        self as *const Self as usize
    }
}

pub struct ListFrameAllocator {
    phys_mem_offset: u64,
    next_free: Option<&'static mut FrameInfo>,
}

impl ListFrameAllocator {
    /// Create a `FrameAllocator` from the passed memory map.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that the passed memory map is valid.
    /// The main requirement is that all frames marked as `USABLE` are
    /// really unused.
    pub unsafe fn init(boot_info: &'static BootInfo) -> Self {
        let phys_mem_offset = boot_info.physical_memory_offset;
        let regions = boot_info.memory_map.iter();
        let usable_regions = regions.filter(|r| r.region_type == MemoryRegionType::Usable);
        let addr_ranges = usable_regions.map(|r| r.range.start_addr()..r.range.end_addr());
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
        let mut frame_infos = frame_addresses.map(|addr| {
            PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(addr))
                .start_address()
                .as_u64()
                + phys_mem_offset
        });

        let first_ptr = frame_infos.next().unwrap() as *mut FrameInfo;
        // This fails because we're already in virtual memory. Crap.
        first_ptr.write(FrameInfo { next: None });

        let first_frame = &mut *first_ptr;
        let mut current = &mut *first_ptr;

        for frame_ptr in frame_infos {
            let frame_ptr = frame_ptr as *mut FrameInfo;
            frame_ptr.write(FrameInfo { next: None });

            current.next = Some(&mut *frame_ptr);
            current = &mut *frame_ptr;
        }

        ListFrameAllocator {
            phys_mem_offset,
            next_free: Some(first_frame),
        }
    }
}

unsafe impl FrameAllocator<Size4KiB> for ListFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        match self.next_free.take() {
            None => None,
            Some(frame_info) => {
                self.next_free = frame_info.next.take();
                let phys_addr = frame_info.start_addr() as u64 - self.phys_mem_offset;
                unsafe {
                    Some(PhysFrame::from_start_address_unchecked(
                        PhysAddr::new_unsafe(phys_addr),
                    ))
                }
            }
        }
    }
}

impl FrameDeallocator<Size4KiB> for ListFrameAllocator {
    unsafe fn deallocate_frame(&mut self, frame: PhysFrame<Size4KiB>) {
        let virt_addr = frame.start_address().as_u64() + self.phys_mem_offset;
        let frame_ptr = virt_addr as *mut FrameInfo;
        frame_ptr.write(FrameInfo {
            next: self.next_free.take(),
        });
        self.next_free = Some(&mut *frame_ptr);
    }
}
