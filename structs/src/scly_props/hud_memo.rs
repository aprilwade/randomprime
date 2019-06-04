use auto_struct_macros::auto_struct;

use crate::SclyPropertyData;
use reader_writer::CStr;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct HudMemo<'r>
{
    #[auto_struct(expect = 6)]
    prop_count: u32,

    pub name: CStr<'r>,

    pub first_message_timer: f32,
    pub unknown: u8,
    pub memo_type: u32,
    pub strg: u32,
    pub active: u8,
}

impl<'r> SclyPropertyData for HudMemo<'r>
{
    const OBJECT_TYPE: u8 = 0x17;
}
