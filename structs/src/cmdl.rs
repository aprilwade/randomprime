use auto_struct_macros::auto_struct;

use reader_writer::{LazyArray, RoArray, RoArrayIter, IteratorArray};
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;

use crate::ResId;
use crate::res_id::*;

// We don't need to modify CMDLs, so most of the details are left out.
// We only actually care about reading out the TXTR file ids.
#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Cmdl<'r>
{
    #[auto_struct(expect = 0xDEADBABE)]
    magic: u32,

    #[auto_struct(expect = 2)]
    version: u32,

    pub flags: u32,

    pub maab: GenericArray<f32, U6>,

    pub data_section_count: u32,
    pub material_set_count: u32,

    // TODO: Iter derive
    #[auto_struct(init = (material_set_count as usize, ()))]
    pub material_set_sizes: RoArray<'r, u32>,
    #[auto_struct(init = ((data_section_count - material_set_count) as usize, ()))]
    pub data_section_sizes: RoArray<'r, u32>,

    #[auto_struct(pad_align = 32)]
    _pad: (),

    #[auto_struct(init = material_set_sizes.iter())]
    pub material_sets: IteratorArray<'r, CmdlMaterialSet<'r>, RoArrayIter<'r, u32>>,
    #[auto_struct(init = data_section_sizes.iter())]
    pub data_sections: IteratorArray<'r, CmdlDataSection<'r>, RoArrayIter<'r, u32>>,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct CmdlMaterialSet<'r>
{
    #[auto_struct(args)]
    size: u32,

    #[auto_struct(derive = texture_ids.len() as u32)]
    pub texture_count: u32,
    #[auto_struct(init = (texture_count as usize, ()))]
    pub texture_ids: LazyArray<'r, ResId<TXTR>>,

    #[auto_struct(init = (size as usize - 4 - texture_ids.size(), ()))]
    pub remainder: RoArray<'r, u8>,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct CmdlDataSection<'r>
{
    #[auto_struct(args)]
    size: u32,

    #[auto_struct(init = (size as usize, ()))]
    pub remainder: RoArray<'r, u8>,
}
