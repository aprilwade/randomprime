use auto_struct_macros::auto_struct;

use reader_writer::{CStr, FourCC, RoArray};

use crate::ResId;
use crate::res_id::*;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Font<'r>
{
    #[auto_struct(expect = FourCC::from_bytes(b"FONT"))]
    magic: FourCC,

    #[auto_struct(expect = 2)]
    version: u32,

    pub unknown0: u32,
    pub line_height: u32,
    pub vertical_offset: u32,
    pub line_margin: u32,
    pub unknown1: u8,
    pub unknown2: u8,
    pub unknown3: u32,
    pub font_size: u32,

    pub name: CStr<'r>,
    pub txtr: ResId<TXTR>,
    pub txtr_fmt: u32,

    #[auto_struct(derive = glyphs.len() as u32)]
    glyph_count: u32,
    #[auto_struct(init = (glyph_count as usize, ()))]
    glyphs: RoArray<'r, FontGlyph>,

    #[auto_struct(derive = kernings.len() as u32)]
    kerning_count: u32,
    #[auto_struct(init = (kerning_count as usize, ()))]
    kernings: RoArray<'r, FontKerning>,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct FontGlyph
{
    utf16_char: u16,
    left_uv_coordinate: f32,
    top_uv_coordinate: f32,
    right_uv_coordinate: f32,
    bottom_uv_coordinate: f32,
    left_padding: u32,
    print_head_advance: u32,
    right_padding: u32,
    width: u32,
    height: u32,
    vertical_offset: u32,
    kerning_start_index: u32,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct FontKerning
{
    char1: u16,
    char2: u16,
    kerning_adjust: i32,
}
