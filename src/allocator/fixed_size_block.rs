use super::linked_list;
use super::locked::Locked;
use alloc::alloc::{GlobalAlloc, Layout};
use core::mem;
use core::ptr::{self, NonNull};

// Every node in a list has the same size, so we don't need
// to keep track
struct Node {
    next: Option<&'static mut Node>,
}

/// The block sizes to use.
///
/// Each must be a power of two because they're also used as the block's
/// alignment.
const BLOCK_SIZES: &[usize] = &[8, 16, 32, 64, 128, 256, 512, 1024, 2048];
const LAST_BIN: usize = BLOCK_SIZES.len() - 1;

#[inline]
fn list_index(layout: &Layout) -> Option<usize> {
    let required_block_size = layout.size().max(layout.align());
    BLOCK_SIZES.iter().position(|&s| s >= required_block_size)
}

pub struct Allocator {
    list_heads: [Option<&'static mut Node>; BLOCK_SIZES.len()],
    fallback: linked_list::Allocator,
}

impl Allocator {
    /// Create a new, empty `Allocator` with an empty fallback.
    #[allow(dead_code)]
    pub const fn empty() -> Self {
        const EMPTY: Option<&'static mut Node> = None;
        Self {
            list_heads: [EMPTY; BLOCK_SIZES.len()],
            fallback: linked_list::Allocator::empty(),
        }
    }

    /// Initialize the allocator with the provided heap bounds.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that the provided heap bounds are invalid
    /// and that the memory is unused.
    #[allow(dead_code)]
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.fallback.init(heap_start, heap_size);
    }

    /// Allocate a region using the fallback allocator
    unsafe fn fallback_alloc(&mut self, layout: Layout) -> *mut u8 {
        match self.fallback.alloc_first_fit(layout) {
            Ok(p) => p.as_ptr(),
            Err(_) => ptr::null_mut(),
        }
    }
}

unsafe impl GlobalAlloc for Locked<Allocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut allocator = self.lock();
        match list_index(&layout) {
            Some(i) => match allocator.list_heads[i].take() {
                Some(node) => {
                    allocator.list_heads[i] = node.next.take();
                    node as *mut Node as *mut u8
                }
                None => {
                    // TODO check if we have a larger block we can split
                    if i < LAST_BIN {
                        if let Some(ref mut head) = allocator.list_heads[i + 1].take() {
                            // Split a larger block
                            let start_addr = ptr::addr_of!(head) as usize;
                            let second_ptr = (start_addr + BLOCK_SIZES[i]) as *mut Node;
                            second_ptr.write(Node { next: None });
                            allocator.list_heads[i] = Some(&mut *second_ptr);
                            return start_addr as *mut u8;
                        }
                    }
                    let block_size = BLOCK_SIZES[i];
                    let block_align = block_size;
                    let layout = Layout::from_size_align(block_size, block_align).unwrap();
                    allocator.fallback_alloc(layout)
                }
            },
            None => match allocator.fallback.alloc_first_fit(layout) {
                Ok(p) => p.as_ptr(),
                Err(linked_list::AllocError::OOM) => {
                    // Empty all our bins in the hope of success
                    // This will have _awful_ worst case performance, but it will stave off failure.
                    BLOCK_SIZES.iter().enumerate().for_each(|(i, block_size)| {
                        let layout = Layout::from_size_align(*block_size, *block_size).unwrap();
                        while let Some(ref mut head) = allocator.list_heads[i].take() {
                            let block_addr = ptr::addr_of!(head) as *mut u8;
                            let ptr = NonNull::new(block_addr).unwrap();
                            allocator.fallback.deallocate(ptr, layout);
                        }
                    });
                    allocator.fallback_alloc(layout)
                }
                Err(_) => ptr::null_mut(),
            },
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut allocator = self.lock();
        match list_index(&layout) {
            Some(i) => {
                assert!(mem::size_of::<Node>() <= BLOCK_SIZES[i]);
                assert!(mem::align_of::<Node>() <= BLOCK_SIZES[i]);
                let node_ptr = ptr as *mut Node;
                node_ptr.write(Node {
                    next: allocator.list_heads[i].take(),
                });
                allocator.list_heads[i] = Some(&mut *node_ptr);
            }
            None => {
                let ptr = NonNull::new(ptr).unwrap();
                allocator.fallback.deallocate(ptr, layout);
            }
        }
    }
}
