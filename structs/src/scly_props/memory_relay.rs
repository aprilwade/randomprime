use auto_struct_macros::auto_struct;

use crate::SclyPropertyData;
use reader_writer::CStr;


#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct MemoryRelay<'r>
{
    #[auto_struct(expect = 3)]
    prop_count: u32,

    pub name: CStr<'r>,
    pub unknown: u8,
    pub active: u8,
}

impl<'r> SclyPropertyData for MemoryRelay<'r>
{
    const OBJECT_TYPE: u8 = 0x13;
}
