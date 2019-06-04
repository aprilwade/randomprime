use auto_struct_macros::auto_struct;

use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use crate::SclyPropertyData;

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct PlayerHintStruct
{
    #[auto_struct(expect = 15)]
    prop_count: u32,

    // 15 unknowns, left out for simplicity
    pub unknowns: GenericArray<u8, U15>,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct PlayerHint<'r>
{
    #[auto_struct(expect = 6)]
    prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,

    pub unknown0: u8,

    pub inner_struct: PlayerHintStruct,

    pub unknown1: u32,
}

impl<'r> SclyPropertyData for PlayerHint<'r>
{
    const OBJECT_TYPE: u8 = 0x3E;
}
