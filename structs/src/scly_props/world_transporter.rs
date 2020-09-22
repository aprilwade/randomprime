use auto_struct_macros::auto_struct;

use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use crate::{ResId, SclyPropertyData};
use crate::res_id::*;
use crate::scly_props::structs::AncsProp;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct WorldTransporter<'r>
{
    #[auto_struct(derive = 21 + 5 * pal_additions.is_some() as u32)]
    prop_count: u32,

    pub name: CStr<'r>,

    pub active: u8,
    pub mlvl: ResId<MLVL>,
    pub mrea: ResId<MREA>,
    pub ancs: AncsProp,
    pub player_scale: GenericArray<f32, U3>,
    pub platform_model: ResId<CMDL>,
    pub platform_scale: GenericArray<f32, U3>,
    pub background_model: ResId<CMDL>,
    pub background_scale: GenericArray<f32, U3>,
    pub up_elevator: u8,
    pub elevator_sound: u32,
    pub volume: u32,
    pub panning: u32,
    pub show_text: u8,
    pub font: ResId<FONT>,
    pub strg: ResId<STRG>,
    pub fade_white: u8,
    pub char_fade_in_time: f32,
    pub chars_per_second: f32,
    pub show_delay: f32,

    #[auto_struct(init = if prop_count == 26 { Some(()) } else { None })]
    pub pal_additions: Option<WorldTransporterPalAdditions<'r>>
}


#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct WorldTransporterPalAdditions<'r>
{
    pub audio_stream: CStr<'r>,
    pub unknown0: u8,
    pub unknown1: f32,
    pub unknown2: f32,
    pub unknown3: f32,
}

impl<'r> SclyPropertyData for WorldTransporter<'r>
{
    const OBJECT_TYPE: u8 = 0x062;
}
