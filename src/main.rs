#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

const VGA_BUFFER: *mut u8 = 0xB8000;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {}
}
