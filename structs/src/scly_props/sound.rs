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
    pub sound_id: u32,

    pub active: u8,
    pub max_dist: f32,
    pub dist_comp: f32,
    pub start_delay: f32,
    pub min_volume: u32,
    pub volume: u32,
    pub priority: u32,
    pub pan: u32,
    pub loops: u8,
    pub non_emitter: u8,
    pub auto_start: u8,
    pub occlusion_test: u8,
    pub acoustics: u8,
    pub world_sfx: u8,
    pub allow_duplicates: u8,
    pub pitch: u32,
}

impl<'r> SclyPropertyData for Sound<'r>
{
    const OBJECT_TYPE: u8 = 0x9;
}
