#![no_std]
#![no_main]

use firstos::vga;

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

const VGA_BUFFER: *mut u8 = 0xB8000 as *mut u8;
const MSG: &[u8] = b"We've booted!";

#[no_mangle]
pub extern "C" fn _start() -> ! {
    for (i, &byte) in MSG.iter().enumerate() {
        unsafe {
            *VGA_BUFFER.offset(i as isize * 2) = byte;
            *VGA_BUFFER.offset(i as isize * 2 + 1) = 0xb;
        }
    }
    loop {}
}
