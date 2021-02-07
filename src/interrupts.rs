use lazy_static::lazy_static;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use crate::println;
use crate::gdt;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        unsafe {
            idt.double_fault.set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_INDEX);
        }
        idt
    };
}

pub fn init_idt() {
    IDT.load();
}

extern "x86-interrupt" fn breakpoint_handler(
    stack_frame: &mut InterruptStackFrame
) {
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

// The error code is aalways 0, so ignore it
extern "x86-interrupt" fn double_fault_handler(
    stack_frame: &mut InterruptStackFrame, _error_code: u64
) -> !{
    panic!("EXCEPTION: DOUBLE THE FAULTS DOUBLE THE FUN\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn page_fault_handler(stack: &mut InterruptStackFrame, error_code: PageFaultErrorCode) {
    panic!("EXCEPTION: FAULTY PAGE, YOU SHOULD FIRE THEM\n{:#?}\n{:#?}", stack, error_code);
}


#[test_case]
fn test_breakpoint_exception() {
    // invoke a breakpoint exception
    x86_64::instructions::interrupts::int3();
}