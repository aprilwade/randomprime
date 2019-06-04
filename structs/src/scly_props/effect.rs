use auto_struct_macros::auto_struct;

use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;

use crate::SclyPropertyData;
use crate::scly_props::structs::LightParameters;


#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Effect<'r>
{
    #[auto_struct(expect = 24)]
    prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,

    pub part: u32,
    pub elsc: u32,

    pub unknown0: u8,
    pub unknown1: u8,
    pub unknown2: u8,
    pub unknown3: u8,
    pub unknown4: u8,
    pub unknown5: f32,
    pub unknown6: f32,
    pub unknown7: f32,
    pub unknown8: f32,
    pub unknown9: u8,
    pub unknown10: f32,
    pub unknown11: f32,
    pub unknown12: f32,
    pub unknown13: u8,
    pub unknown14: u8,
    pub unknown15: u8,
    pub unknown16: u8,

    pub light_params: LightParameters,
}

impl<'r> SclyPropertyData for Effect<'r>
{
    const OBJECT_TYPE: u8 = 0x7;
}
