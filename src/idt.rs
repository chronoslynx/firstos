use bit_field::BitField;
use crate::structures::PrivilegeLevel;

#[repr(C)]
#[derive(Debug,Clone,Copy,PartialEq)]
pub struct Entry<F> {
    ptr_low: u16,
    gdt_selector: u16,
    options: EntryOptions,
    ptr_middle: u16,
    ptr_high: u32,
    reserved: u32,
    phantom: PhantomData<F>,
}

/*
 * Bits   Name                             Description
 * 0-2    Interrupt Stack Table Index      0: Don't switch stacks, 1-7: Switch to the n-th
 *                                         stack in the Interrupt Stack Table when this
 *                                         handler is called.
 * 3-7    Reserved
 * 8      0: Interrupt Gate,               If this bit is 0, interrupts are disabled when
 *        1: Trap Gate                     this handler is called.
 * 9-11   must be one
 * 12     must be zero
 * 13‑14  Descriptor Privilege Level (DPL) The minimal privilege level required for calling
 *                                         this handler.
 * 15    Present
*/
#[repr(transparent)]
#[derive(Debug,Clone,Copy,PartialEq)]
pub struct EntryOptions(u16);

impl EntryOptions {
    #[inline]
    fn minimal() -> Self {
        // Set bits 9-11 and leave the rest empty
        EntryOptions(0b1110_0000_0000)
    }   

    #[inline]
    fn disable_interrupts(&mut self, disable: bool) -> &mut Self {
        self.0.set_bit(8, !disable);
        self
    }

    #[inline]
    fn set_privilege_level(&mut self, dpl: PrivilegeLevel) -> &mut Self {
        self.0.set_bits(13..15, dpl as u16);
        self
    }
    
    /// This is unsafe as the caller must ensure the stack index is valid
    #[inline]
    pub unsafe fn set_stack_index(&mut self, index: u16) -> &mut Self {
        self.0.set_bits(0..3, index + 1);
        self
    }
}
