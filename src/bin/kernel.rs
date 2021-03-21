#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(firstos::test_runner)]
#![reexport_test_harness_main = "test_main"]

use bootloader::BootInfo;
use firstos::{self, println};
use x86_64::VirtAddr;

use core::panic::PanicInfo;

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    firstos::eprint!("{}", info);
    firstos::hlt_loop();
}

const MSG: &str = "We've booted! Hooray!";

bootloader::entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use firstos::memory::{self, BootInfoFrameAllocator};
    use x86_64::structures::paging::Page;
    firstos::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe {memory::init(phys_mem_offset)};
    let mut frame_allocator = unsafe {BootInfoFrameAllocator::init(&boot_info.memory_map)};
    let page = Page::containing_address(VirtAddr::new(0));
    memory::create_example_mapping(page, &mut mapper, &mut frame_allocator);

    let page_ptr: *mut u64 = page.start_address().as_mut_ptr();
    unsafe {
        // write `New!` to the screen
        page_ptr.offset(400).write_volatile(0x_f021_f077_f065_f64e);
    }

    #[cfg(test)]
    test_main();

    println!("{}", MSG);

    firstos::hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    firstos::test_panic_handler(info)
}
