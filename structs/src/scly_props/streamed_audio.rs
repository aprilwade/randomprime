use auto_struct_macros::auto_struct;

use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use crate::SclyPropertyData;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct StreamedAudio<'r>
{
    #[auto_struct(expect = 9)]
    prop_count: u32,

    pub name: CStr<'r>,

    pub active: u8,
    pub audio_file_name: CStr<'r>,

    // 6 unknown properties
    pub unknowns: GenericArray<u8, U18>,
}

impl<'r> SclyPropertyData for StreamedAudio<'r>
{
    const OBJECT_TYPE: u8 = 0x61;
}
