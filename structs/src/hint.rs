use auto_struct_macros::auto_struct;
use reader_writer::{CStr, LazyArray};

use crate::ResId;
use crate::res_id::*;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Hint<'r>
{
    #[auto_struct(expect = 0x00BADBAD)]
    magic: u32,
    #[auto_struct(expect = 1)]
    version: u32,

    #[auto_struct(derive = hints.len() as u32)]
    pub hint_count: u32,
    #[auto_struct(init = (hint_count as usize, ()))]
    pub hints: LazyArray<'r, HintDetails<'r>>,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct HintDetails<'r>
{
    pub hint_name: CStr<'r>,
    pub intermediate_time: f32,
    pub normal_time: f32,
    pub popup_text_strg: u32,
    pub text_time: u32,

    #[auto_struct(derive = locations.len() as u32)]
    pub location_count: u32,
    #[auto_struct(init = (location_count as usize, ()))]
    pub locations: LazyArray<'r, HintLocation>,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct HintLocation
{
    pub mlvl: ResId<MLVL>,
    pub mrea: ResId<MREA>,
    pub target_room_index: u32,
    pub map_text_strg: ResId<STRG>,
}
