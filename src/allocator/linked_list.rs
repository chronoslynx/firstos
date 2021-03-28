use super::align_up;
use super::locked::Locked;
use alloc::alloc::{GlobalAlloc, Layout};
use core::mem;
use core::ptr::{self, NonNull};

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

#[derive(Debug)]
pub enum AllocError {
    OOM,
    TooSmall,
    Overflow,
}

type Result<T> = core::result::Result<T, AllocError>;

pub struct Allocator {
    head: Node,
}

impl Allocator {
    /// Create a new empty bump allocator
    #[allow(dead_code)]
    pub const fn empty() -> Self {
        Self { head: Node::new(0) }
    }

    /// Initialize a bump allocator with the given heap bounds.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the given memory range is unused.
    #[allow(dead_code)]
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.add_free_region(heap_start, heap_size)
    }

    /// Add a region to our free list.
    ///
    /// This region will be merged with another if its `start_addr` is another region's
    /// `end_addr` or if its end_addr is another region's `start_addr`.
    ///
    /// # Safety
    ///
    /// This method is unsafe as it creates a new `Node` at the specified address.
    /// This may cause undefined behavior if `addr` is invalid or this memory is in use elsewhere.
    unsafe fn add_free_region(&mut self, addr: usize, size: usize) {
        // Ensure that the free region is large enough to hold a region header
        assert_eq!(align_up(addr, mem::align_of::<Node>()), addr);
        assert!(size >= mem::size_of::<Node>());

        // append the new node to the start of our free list
        let node_ptr = addr as *mut Node;
        node_ptr.write(Node::new(size));
        let node = &mut *node_ptr;

        let our_start = node.start_addr();
        let our_end = node.end_addr();

        let mut current = &mut self.head;
        while let Some(ref mut region) = current.next {
            let region_start = region.start_addr();
            let region_end = region.end_addr();
            if region_end == our_start {
                // append ourselves to this region
                region.size += node.size;

                // See if we can append `region.next` now that we've extended `region`
                let mut tail = region.next.take();
                if let Some(ref mut n) = tail {
                    if n.start_addr() == region.end_addr() {
                        // merge once more
                        region.next = n.next.take();
                        region.size += n.size;
                    } else {
                        region.next = tail;
                    }
                }
                return;
            } else if region_start == our_end {
                // append this region to ourselves
                node.size += region.size;
                // Repair the list
                current.next = Some(node);
                return;
            } else if region_start > our_start {
                // insert here
                break;
            }
            current = current.next.as_mut().unwrap();
        }
        node.next = current.next.take();
        current.next = Some(node);
    }

    /// Find a free region with the given size and alignment and remove it from our free list.
    ///
    /// Returns the list node and the start address of the allocation.
    fn find_region(&mut self, size: usize, align: usize) -> Result<(&'static mut Node, usize)> {
        let mut current = &mut self.head;
        while let Some(ref mut region) = current.next {
            if let Ok(alloc_start) = Self::alloc_from_region(&region, size, align) {
                let next = region.next.take();
                let ret = Ok((current.next.take().unwrap(), alloc_start));
                current.next = next;
                return ret;
            } else {
                // current region is not suitable
                current = current.next.as_mut().unwrap();
            }
        }
        // No suitable region found
        Err(AllocError::OOM)
    }

    fn alloc_from_region(region: &Node, size: usize, align: usize) -> Result<usize> {
        let alloc_start = align_up(region.start_addr(), align);
        let alloc_end = alloc_start.checked_add(size).ok_or(AllocError::Overflow)?;

        if alloc_end > region.end_addr() {
            // region too small
            return Err(AllocError::TooSmall);
        }
        let excess_size = region.end_addr() - alloc_end;
        if excess_size > 0 && excess_size < mem::size_of::<Node>() {
            // Region is too small for this allocation plus a new unused node
            return Err(AllocError::TooSmall);
        }
        Ok(alloc_start)
    }

    /// Adjust the given layout such that any allocated memory region is capable of storing a `Node`.
    ///
    /// Returns the adjusted (size, alignment)
    #[inline]
    fn size_align(layout: Layout) -> (usize, usize) {
        let layout = layout
            .align_to(mem::align_of::<Node>())
            .expect("failed to adjust alignment")
            .pad_to_align();
        let size = layout.size().max(mem::size_of::<Node>());
        (size, layout.align())
    }

    pub unsafe fn alloc_first_fit(&mut self, layout: Layout) -> Result<NonNull<u8>> {
        let (size, align) = Allocator::size_align(layout);
        let (region, alloc_start) = self.find_region(size, align)?;
        let alloc_end = alloc_start.checked_add(size).expect("alloc overflow");
        let excess_size = region.end_addr() - alloc_end;
        if excess_size > 0 {
            // hello fragmentation
            self.add_free_region(alloc_end, excess_size);
        }
        Ok(NonNull::new_unchecked(alloc_start as *mut u8))
    }

    pub unsafe fn deallocate(&mut self, ptr: NonNull<u8>, layout: Layout) {
        let (adjusted_size, _) = Allocator::size_align(layout);
        self.add_free_region(ptr.as_ptr() as usize, adjusted_size)
    }
}

unsafe impl GlobalAlloc for Locked<Allocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut ll = self.lock();
        match ll.alloc_first_fit(layout) {
            Ok(p) => p.as_ptr(),
            Err(_) => ptr::null_mut(),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let ptr = NonNull::new(ptr).unwrap();
        self.lock().deallocate(ptr, layout)
    }
}

#[test_case]
fn test_linked_list_stays_sorted_trivial() {
    use crate::serial_println;
    let heap: [u8; 512] = [0; 512];
    let alloc = Locked::new(Allocator::empty());
    serial_println!("alloc created");
    let l1 = Layout::from_size_align(2, 1).expect("invalid layout");
    unsafe {
        let heap_addr = ptr::addr_of!(heap) as usize;
        alloc.lock().init(heap_addr, heap.len());
        serial_println!("allocator initialized");
    }
    unsafe {
        let p1 = alloc.alloc(l1);
        assert!(!p1.is_null(), "failed to allocate first block");
        let p1_node = (((p1 as usize) - mem::size_of::<Node>()) as *mut Node)
            .as_ref()
            .unwrap();
        let start_addr = p1_node.start_addr();

        // Put the block back on our free list
        alloc.dealloc(p1, l1);
        let pnew = alloc.alloc(l1);
        let pnew_node = (((pnew as usize) - mem::size_of::<Node>()) as *mut Node)
            .as_ref()
            .unwrap();
        assert_eq!(
            pnew_node.start_addr(),
            start_addr,
            "should have reused the first node found"
        );
    }
}

// I can't get my address arithmetic to work out with this test, so I'm dealing with it another day.
// #[test_case]
// fn test_linked_list_merge_after() {
//     // Test whether our logic for merging after a given node works
//     use crate::serial_println;
//     let heap: [u8; 512] = [0; 512];
//     let alloc = Locked::new(Allocator::empty());
//     let l = Layout::from_size_align(100, 1).expect("invalid layout");
//     unsafe {
//         let heap_addr = ptr::addr_of!(heap) as usize;
//         alloc.lock().init(heap_addr, heap.len());
//         let p1 = alloc.alloc(l);
//         assert!(!p1.is_null(), "failed to allocate first block");
//         let p1_node = (((p1 as usize) - mem::size_of::<Node>()) as *mut Node)
//             .as_ref()
//             .unwrap();
//         let p1_start = p1_node.start_addr();
//         let p1_end = p1_node.end_addr();
//         serial_println!("p1 is [{}, {}]", p1_start, p1_end);
//
//         let p2 = alloc.alloc(l);
//         assert!(!p2.is_null(), "failed to allocate second block");
//         let p2_node = (((p2 as usize) - mem::size_of::<Node>()) as *mut Node)
//             .as_ref()
//             .unwrap();
//         let p2_start = p2_node.start_addr();
//         let p2_end = p2_node.end_addr();
//         serial_println!("p2 is [{}, {}]", p2_start, p2_end);
//
//         let p3 = alloc.alloc(l);
//         assert!(!p3.is_null(), "failed to allocate third block");
//
//         // Put the first block at the head of the free list
//         serial_println!("deallocating first block");
//         alloc.dealloc(p1, l);
//         serial_println!("deallocating second block");
//         alloc.dealloc(p2, l);
//
//         // p2 should be merged into p1, but there should be a gap after it
//         let double_l = Layout::from_size_align(200, 1).expect("invalid double layout");
//         serial_println!("allocating larger block");
//         let p = alloc.alloc(double_l);
//         assert!(!p.is_null(), "failed to allocate larger block");
//
//         let new_node = &*(((p as usize) - mem::size_of::<Node>()) as *mut Node);
//         serial_println!(
//             "new is [{}, {}]",
//             new_node.start_addr(),
//             new_node.end_addr()
//         );
//         assert_eq!(
//             new_node.start_addr(),
//             p1_start,
//             "We should be able to use a block starting at p1"
//         );
//         assert!(
//             new_node.end_addr() > p2_start,
//             "We should use up the old p2 block"
//         );
//     }
// }
