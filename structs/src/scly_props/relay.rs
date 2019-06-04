use auto_struct_macros::auto_struct;

use reader_writer::CStr;
use crate::SclyPropertyData;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Relay<'r>
{
    #[auto_struct(expect = 2)]
    prop_count: u32,

    pub name: CStr<'r>,

    pub active: u8,
}

impl<'r> SclyPropertyData for Relay<'r>
{
    const OBJECT_TYPE: u8 = 0x15;
}
