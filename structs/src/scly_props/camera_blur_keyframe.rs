use auto_struct_macros::auto_struct;

use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use crate::SclyPropertyData;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct CameraBlurKeyframe<'r>
{
    #[auto_struct(expect = 8)]
    pub prop_count: u32,

    pub name: CStr<'r>,
    pub active: u8,
    pub unknowns: GenericArray<u8, U5>,
}

impl<'r> SclyPropertyData for CameraBlurKeyframe<'r>
{
    const OBJECT_TYPE: u8 = 0x19;
}
