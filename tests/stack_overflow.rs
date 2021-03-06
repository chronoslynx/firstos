#![feature(abi_x86_interrupt)]
#![no_std]
#![no_main]

use core::panic::PanicInfo;
use firstos::{serial_print,qemu, serial_println};
use x86_64::structures::idt::InterruptStackFrame;
use lazy_static::lazy_static;
use x86_64::structures::idt::InterruptDescriptorTable;

#[no_mangle] // don't mangle the name of this function
pub extern "C" fn _start() -> ! {
    serial_print!("stack_overflow::stack_overflow...\t");
    firstos::gdt::init();
    init_test_idt();
    stack_overflow();

    panic!("Execution continued after stack overflow");
}

#[allow(unconditional_recursion)]
fn stack_overflow() {
    stack_overflow();
    unsafe {
        let ptr = 0 as *mut u8;
        ptr.read_volatile();
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    firstos::test_panic_handler(info)
}

lazy_static! {
    static ref TEST_IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        unsafe {
            idt.double_fault
                .set_handler_fn(test_double_fault_handler)
                .set_stack_index(firstos::gdt::DOUBLE_FAULT_IST_INDEX);
        }

        idt
    };
}

pub fn init_test_idt() {
    TEST_IDT.load();
}

extern "x86-interrupt" fn test_double_fault_handler(
    _stack_frame: &mut InterruptStackFrame,
    _error_code: u64
) -> ! {
    serial_println!("ok");
    qemu::exit(qemu::ExitCode::Success);
    loop{}
}
