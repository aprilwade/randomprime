use auto_struct_macros::auto_struct;

use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use crate::scly_props::structs::ScannableParameters;
use crate::SclyPropertyData;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct PointOfInterest<'r>
{
    #[auto_struct(expect = 6)]
    prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub unknown0: u8,
    pub scan_param: ScannableParameters,
    pub unknown1: f32,
}

impl<'r> SclyPropertyData for PointOfInterest<'r>
{
    const OBJECT_TYPE: u8 = 0x42;
}
