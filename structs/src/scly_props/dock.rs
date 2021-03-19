use auto_struct_macros::auto_struct;

use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use crate::SclyPropertyData;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Dock<'r>
{
    #[auto_struct(expect = 7)]
    prop_count: u32,

    pub name: CStr<'r>,

    pub unknown0: u8,
    pub position: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,
    pub dock_number: u32,
    pub this_room: u8,
    pub unknown1: u8,
}

impl<'r> SclyPropertyData for Dock<'r>
{
    const OBJECT_TYPE: u8 = 0x0B;
}
