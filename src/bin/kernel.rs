#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(firstos::test_runner)]
#![reexport_test_harness_main = "test_main"]

use firstos::{self, eprint, println};

use core::panic::PanicInfo;

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    eprint!("{}", info);
    firstos::hlt_loop();
}

const MSG: &str = "We've booted! Hooray!";

#[no_mangle]
pub extern "C" fn _start() -> ! {
    firstos::init();
        // new
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
