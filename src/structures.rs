#[repr(u8)]
#[derive(Debug,Copy,PartialEq,Eq)]
pub enum PrivilegeLevel {
    Ring0,
    Ring1,
    Ring2,
    Ring3
}
