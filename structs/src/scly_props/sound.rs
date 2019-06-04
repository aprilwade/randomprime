use auto_struct_macros::auto_struct;

use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use crate::SclyPropertyData;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Sound<'r>
{
    #[auto_struct(expect = 20)]
    prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,

    // 17 unknown properties
    pub unknowns: GenericArray<u8, U44>,
}

impl<'r> SclyPropertyData for Sound<'r>
{
    const OBJECT_TYPE: u8 = 0x9;
}
