use auto_struct_macros::auto_struct;

use reader_writer::CStr;
use crate::SclyPropertyData;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Timer<'r>
{
    #[auto_struct(expect = 6)]
    prop_count: u32,

    pub name: CStr<'r>,

    pub start_time: f32,
    pub max_random_add: f32,
    pub reset_to_zero: u8,
    pub start_immediately: u8,
    pub active: u8,
}

impl<'r> SclyPropertyData for Timer<'r>
{
    const OBJECT_TYPE: u8 = 0x5;
}
