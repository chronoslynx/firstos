mod bump;
mod fixed_size_block;
mod linked_list;
mod locked;

use locked::Locked;
use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB,
    },
    VirtAddr,
};

pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 100 * 1024; // 100 KiB

#[cfg(feature = "heap_fixed_block")]
#[global_allocator]
static ALLOCATOR: Locked<fixed_size_block::Allocator> =
    Locked::new(fixed_size_block::Allocator::empty());

#[cfg(feature = "heap_linked_list")]
#[global_allocator]
static ALLOCATOR: Locked<linked_list::Allocator> = Locked::new(linked_list::Allocator::empty());

#[cfg(feature = "heap_bump")]
#[global_allocator]
static ALLOCATOR: Locked<bump::Allocator> = Locked::new(bump::Allocator::empty());

/// Align the given address `addr` upwards to alignment `align`.
///
/// Requires that `align` is a power of two, which is guaranteed
/// by the `GlobalAlloc` trait.
#[inline(always)]
fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

pub fn init(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError<Size4KiB>> {
    let page_range = {
        let heap_start = VirtAddr::new(HEAP_START as u64);
        let heap_end = heap_start + HEAP_SIZE - 1u64;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };
    // Allocate a frame to each page
    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe {
            mapper.map_to(page, frame, flags, frame_allocator)?.flush();
        }
    }

    unsafe {
        ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
    }

    Ok(())
}
