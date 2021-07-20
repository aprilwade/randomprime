use auto_struct_macros::auto_struct;

use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use crate::SclyPropertyData;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Waypoint<'r>
{
    #[auto_struct(expect = 13)]
    prop_count: u32,

    pub name: CStr<'r>,
    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub active: u8,
    pub unkown1: f32,
    pub unkown2: f32,
    pub unkown3: u32,
    pub unkown4: u32,
    pub unkown5: u32,
    pub unkown6: u32,
    pub unkown7: u32,
    pub unkown8: u32,
    pub unkown9: u32,
}

impl<'r> SclyPropertyData for Waypoint<'r>
{
    const OBJECT_TYPE: u8 = 0x02;
}
