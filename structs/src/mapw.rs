use auto_struct_macros::auto_struct;

use reader_writer::{LazyArray};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Mapw<'r>
{
    #[auto_struct(expect = 0xDEADF00D)]
    pub magic: u32,
    #[auto_struct(expect = 1)]
    pub version: u32,

    #[auto_struct(derive = area_maps.len() as u32)]
    pub area_map_count: u32,

    #[auto_struct(init = (area_map_count as usize, ()))]
    pub area_maps: LazyArray<'r, u32>,

    #[auto_struct(pad_align = 32)]
    _pad: (),
}
