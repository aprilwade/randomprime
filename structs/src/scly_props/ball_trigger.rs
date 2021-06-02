use auto_struct_macros::auto_struct;

use reader_writer::{
    CStr,
    generic_array::GenericArray,
    typenum::U3,
};
use crate::SclyPropertyData;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct BallTrigger<'r>
{
    #[auto_struct(expect = 9)]
    prop_count: u32,

    pub name: CStr<'r>,
    pub location: GenericArray<f32, U3>,
    pub volume: GenericArray<f32, U3>,
    pub active: u8,
    pub force: f32,
    pub min_angle: f32,
    pub max_distance: f32,
    pub force_angle: GenericArray<f32, U3>,
    pub stop_player: u8,
}

impl<'r> SclyPropertyData for BallTrigger<'r>
{
    const OBJECT_TYPE: u8 = 0x48;
}
