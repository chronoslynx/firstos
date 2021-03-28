use super::locked::Locked;
use alloc::alloc::{GlobalAlloc, Layout};
use core::ptr;

pub struct Allocator {
    heap_start: usize,
    heap_end: usize,
    next: usize,
    allocations: usize,
}

impl Allocator {
    /// Create a new empty bump allocator
    pub const fn empty() -> Self {
        Allocator {
            heap_start: 0,
            heap_end: 0,
            next: 0,
            allocations: 0,
        }
    }

    /// Initialize a bump allocator with the given heap bounds.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the given memory range is unused.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.heap_start = heap_start;
        self.heap_end = heap_start + heap_size;
        self.next = self.heap_end;
    }
}

unsafe impl GlobalAlloc for Locked<Allocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut bump = self.lock();
        let new_ptr = match bump.next.checked_sub(layout.size()) {
            Some(start) => start,
            None => return ptr::null_mut(),
        };
        // Round down to the next alignment
        let alloc_start = new_ptr & !(layout.align() - 1);
        if alloc_start < bump.heap_start {
            ptr::null_mut()
        } else {
            bump.next = alloc_start;
            bump.allocations += 1;
            alloc_start as *mut u8
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut bump = self.lock();
        bump.allocations -= 1;
        let alloc_start = ptr as usize;
        if bump.next == alloc_start {
            // If this was the last allocation we can reuse it immediately
            bump.next = bump.next + layout.size();
        } else if bump.allocations == 0 {
            bump.next = bump.heap_end;
        }
    }
}
