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
    firstos::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);

    let addresses = [
        // the identity-mapped vga buffer page
        0xb8000,
        // some code page
        0x201008,
        // some stack page
        0x0100_0020_1a10,
        // virtual address mapped to physical address 0
        boot_info.physical_memory_offset,
    ];
    for &address in &addresses {
        let virt = VirtAddr::new(address);
        let phys = unsafe { firstos::memory::translate_addr(virt, phys_mem_offset) };
        println!("{:?} -> {:?}", virt, phys);
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
