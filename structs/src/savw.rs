use auto_struct_macros::auto_struct;
use reader_writer::{LazyArray, RoArray};

use crate::ResId;
use crate::res_id::*;

#[auto_struct(Readable, Writable)]
#[derive(Clone, Debug)]
pub struct Savw<'r>
{
    #[auto_struct(expect = 0xC001D00D)]
    magic: u32,

    #[auto_struct(expect = 3)]
    version: u32,

    pub area_count: u32,

    #[auto_struct(derive = cinematic_skip_array.len() as u32)]
    cinematic_skip_count: u32,
    #[auto_struct(init = (cinematic_skip_count as usize, ()))]
    pub cinematic_skip_array: RoArray<'r, u32>,

    #[auto_struct(derive = memory_relay_array.len() as u32)]
    memory_relay_count: u32,
    #[auto_struct(init = (memory_relay_count as usize, ()))]
    pub memory_relay_array: RoArray<'r, u32>,

    #[auto_struct(derive = layer_toggle_array.len() as u32)]
    layer_toggle_count: u32,
    #[auto_struct(init = (layer_toggle_count as usize, ()))]
    pub layer_toggle_array: RoArray<'r, LayerToggle>,

    #[auto_struct(derive = door_array.len() as u32)]
    door_count: u32,
    #[auto_struct(init = (door_count as usize, ()))]
    pub door_array: RoArray<'r, u32>,

    #[auto_struct(derive = scan_array.len() as u32)]
    scan_count: u32,
    #[auto_struct(init = (scan_count as usize, ()))]
    pub scan_array: LazyArray<'r, ScannableObject>,

    #[auto_struct(pad_align = 32)]
    _pad: (),
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Clone, Debug)]
pub struct LayerToggle
{
    pub area_id: u32,
    pub layer_index: u32,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Clone, Debug)]
pub struct ScannableObject
{
    pub scan: ResId<SCAN>,
    pub logbook_category: u32,
}

