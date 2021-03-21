use x86_64::{
    structures::paging::PageTable,
    structures::paging::OffsetPageTable,
    VirtAddr,
    PhysAddr,
};

/// Initialize a new OffsetPageTable.
///
/// This function is unsafe as the caller must guarantee that the entirety
/// of physical memory is mapped to virtual memory at the provided `physical_memory_offset`;
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let l4_table = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(l4_table, physical_memory_offset)
}

/// Returns a mutable reference to the active level 4 page table.
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

/// Translates a given virtual address to its mapped physical address if mapped, otherwise returns None,
pub unsafe fn translate_addr(addr: VirtAddr, physical_memory_offset: VirtAddr) -> Option<PhysAddr> {
    translate_addr_inner(addr, physical_memory_offset)
}

fn translate_addr_inner(addr: VirtAddr, physical_memory_offset: VirtAddr) -> Option<PhysAddr> {
    use x86_64::structures::paging::page_table::FrameError;
    use x86_64::registers::control::Cr3;
    let (l4_frame, _) = Cr3::read();
    let table_indices = [
        addr.p4_index(), addr.p3_index(), addr.p2_index(), addr.p1_index(),
    ];
    let mut frame = l4_frame;
    for &index in &table_indices {
        let virt = physical_memory_offset +frame.start_address().as_u64();
        let table_ptr: *const PageTable = virt.as_ptr();
        let table = unsafe{&*table_ptr};

        // read the PTE and update frame
        let entry = &table[index];
        frame = match entry.frame() {
            Ok(frame) => frame,
            Err(FrameError::FrameNotPresent) => return None,
            Err(FrameError::HugeFrame) => panic!("huge frames not supported"),
        };
    }
    Some(frame.start_address() + u64::from(addr.page_offset()))
}
