#![no_std]
#![no_main]

use firstos::{eprint, println};

use core::panic::PanicInfo;

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
