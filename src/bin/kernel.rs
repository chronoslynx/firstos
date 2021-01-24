#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(firstos::test_runner)]
#![reexport_test_harness_main = "test_main"]

use firstos::{eprint, println};

use core::panic::PanicInfo;

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    eprint!("{}", info);
    loop {}
}

const MSG: &str = "We've booted! Hooray!";

#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("{}", MSG);
    panic!("error handling!");
    loop {}
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    eprint!("testing {}", info);
    loop {}
}
