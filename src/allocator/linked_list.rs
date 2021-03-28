use super::align_up;
use super::locked::Locked;
use alloc::alloc::{GlobalAlloc, Layout};
use core::mem;
use core::ptr;

struct Node {
    size: usize,
    next: Option<&'static mut Node>,
}

impl Node {
    const fn new(size: usize) -> Node {
        Node { size, next: None }
    }

    fn start_addr(&self) -> usize {
        // Start address of our node header
        self as *const Self as usize
    }

    fn end_addr(&self) -> usize {
        self.start_addr() + self.size
    }
}

pub struct Allocator {
    head: Node,
}

impl Allocator {
    /// Create a new empty bump allocator
    pub const fn empty() -> Self {
        Self { head: Node::new(0) }
    }

    /// Initialize a bump allocator with the given heap bounds.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the given memory range is unused.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.add_free_region(heap_start, heap_size)
    }

    unsafe fn add_free_region(&mut self, addr: usize, size: usize) {
        // Ensure that the free region is large enough to hold a region header
        assert_eq!(align_up(addr, mem::align_of::<Node>()), addr);
        assert!(size >= mem::size_of::<Node>());

        // append the new node to the start of our free list
        let mut node = Node::new(size);
        node.next = self.head.next.take();
        let node_ptr = addr as *mut Node;
        node_ptr.write(node);
        self.head.next = Some(&mut *node_ptr);
    }
    /// Find a free region with the given size and alignment and remove it from our free list.
    ///
    /// Returns the list node and the start address of the allocation.
    fn find_region(&mut self, size: usize, align: usize) -> Option<(&'static mut Node, usize)> {
        let mut current = &mut self.head;
        while let Some(ref mut region) = current.next {
            if let Ok(alloc_start) = Self::alloc_from_region(&region, size, align) {
                let next = region.next.take();
                let ret = Some((current.next.take().unwrap(), alloc_start));
                current.next = next;
                return ret;
            } else {
                // current region is not suitable
                current = current.next.as_mut().unwrap();
            }
        }
        None
    }

    fn alloc_from_region(region: &Node, size: usize, align: usize) -> Result<usize, ()> {
        let alloc_start = align_up(region.start_addr(), align);
        let alloc_end = alloc_start.checked_add(size).ok_or(())?;

        if alloc_end > region.end_addr() {
            // region too small
            return Err(());
        }
        let excess_size = region.end_addr() - alloc_end;
        if excess_size > 0 && excess_size < mem::size_of::<Node>() {
            // Region is too small for this allocation plus a new unused node
            // TODO: could do slab/bump allocation within each region to better use memory
            return Err(());
        }
        Ok(alloc_start)
    }

    /// Adjust the given layout such that any allocated memory region is capable of storing a `Node`.
    ///
    /// Returns the adjusted (size, alignment)
    fn size_align(layout: Layout) -> (usize, usize) {
        let layout = layout
            .align_to(mem::align_of::<Node>())
            .expect("failed to adjust alignment")
            .pad_to_align();
        let size = layout.size().max(mem::size_of::<Node>());
        (size, layout.align())
    }
}

unsafe impl GlobalAlloc for Locked<Allocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let (size, align) = Allocator::size_align(layout);
        let mut ll = self.lock();

        if let Some((region, alloc_start)) = ll.find_region(size, align) {
            let alloc_end = alloc_start.checked_add(size).expect("alloc overflow");
            let excess_size = region.end_addr() - alloc_end;
            if excess_size > 0 {
                // hello fragmentation
                ll.add_free_region(alloc_end, excess_size);
            }
            alloc_start as *mut u8
        } else {
            ptr::null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // TODO should be combine regions if they're the same size?
        let (adjusted_size, _) = Allocator::size_align(layout);
        self.lock().add_free_region(ptr as usize, adjusted_size);
    }
}
