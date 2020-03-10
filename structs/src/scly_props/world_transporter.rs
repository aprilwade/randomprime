use auto_struct_macros::auto_struct;

use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use crate::SclyPropertyData;
use crate::scly_props::structs::AncsProp;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct WorldTransporter<'r>
{
    #[auto_struct(derive = 21 + 5 * pal_additions.is_some() as u32)]
    prop_count: u32,

    pub name: CStr<'r>,

    pub unknown0: u8,
    pub mlvl: u32,
    pub mrea: u32,
    pub ancs: AncsProp,
    pub unknown1: GenericArray<f32, U3>,
    pub cmdl0: u32,
    pub unknown2: GenericArray<f32, U3>,
    pub cmdl1: u32,
    pub unknown3: GenericArray<f32, U3>,
    pub unknown4: u8,
    pub unknown5: u32,
    pub unknown6: u32,
    pub unknown7: u32,
    pub unknown8: u8,
    pub font: u32,
    pub strg: u32,
    pub unknown9: u8,
    pub unknown10: f32,
    pub unknown11: f32,
    pub unknown12: f32,

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
