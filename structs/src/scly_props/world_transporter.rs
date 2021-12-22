use auto_struct_macros::auto_struct;

use reader_writer::{
    CStr,
    CStrConversionExtension,
    typenum::*,
    generic_array::GenericArray,
};
use crate::{
    {ResId, SclyPropertyData},
    res_id::*,
    scly_props::structs::AncsProp
};
use std::{
    borrow::Cow,
    ffi::CString
};

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

impl<'r> WorldTransporter<'r>
{
    pub fn warp(mlvl: u32, mrea: u32, teleporter_name: &str, font: ResId<FONT>, strg: ResId<STRG>, is_pal: bool) -> Self
    {
        let pal_additions = if is_pal {
            Some(WorldTransporterPalAdditions {
                audio_stream: b"\0".as_cstr(),
                unknown0: 0,
                unknown1: 0.,
                unknown2: 0.,
                unknown3: 0.,
            })
        } else {
            None
        };
        
        WorldTransporter {
            name: Cow::Owned(CString::new(teleporter_name).unwrap()),
            active: 1,
            mlvl: ResId::new(mlvl),
            mrea: ResId::new(mrea),
            ancs: AncsProp {
                file_id: ResId::invalid(),
                node_index: 0,
                default_animation: 0xFFFFFFFF,
            },
            player_scale: [1., 1., 1.].into(),
            platform_model: ResId::invalid(),
            platform_scale: [1., 1., 1.].into(),
            background_model: ResId::invalid(),
            background_scale: [1., 1., 1.].into(),
            up_elevator: 0,
            elevator_sound: 0xFFFFFFFF,
            volume: 0,
            panning: 0,
            show_text: 1,
            font,
            strg,
            fade_white: 0,
            char_fade_in_time: 1.,
            chars_per_second: 20.,
            show_delay: 1.,
            pal_additions,
        }
    }
}