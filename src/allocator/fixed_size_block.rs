use super::align_up;
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
                    // TODO check if we have smaller blocks we can combine
                    let block_size = BLOCK_SIZES[i];
                    let block_align = block_size;
                    let layout = Layout::from_size_align(block_size, block_align).unwrap();
                    allocator.fallback_alloc(layout)
                }
            },
            None => allocator.fallback_alloc(layout),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut allocator = self.lock();
        match list_index(&layout) {
            Some(i) => {
                let new_node = Node {
                    next: allocator.list_heads[i].take(),
                };
                assert!(mem::size_of::<Node>() <= BLOCK_SIZES[i]);
                assert!(mem::align_of::<Node>() <= BLOCK_SIZES[i]);
                let node_ptr = ptr as *mut Node;
                node_ptr.write(new_node);
                allocator.list_heads[i] = Some(&mut *node_ptr);
            }
            None => {
                let ptr = NonNull::new(ptr).unwrap();
                allocator.fallback.deallocate(ptr, layout);
            }
        }
    }
}
