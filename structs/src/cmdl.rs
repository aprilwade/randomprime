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
    // Header (32-bytes)
    
    #[auto_struct(expect = 0xDEADBABE)]
    magic: u32,
    
    #[auto_struct(expect = 2)]
    version: u32,

    pub flags: u32,
    pub maab: GenericArray<f32, U6>,
    pub data_section_count: u32,
    pub material_set_count: u32,

    #[auto_struct(init = (material_set_count as usize, ()))]
    pub material_set_sizes: RoArray<'r, u32>,

    #[auto_struct(init = (data_section_count as usize, ()))]
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

    // header
    pub texture_count: u32,
    #[auto_struct(init = (texture_count as usize, ()))]
    pub texture_ids: LazyArray<'r, ResId<TXTR>>,

    pub material_count: u32,
    #[auto_struct(init = (material_count as usize, ()))]
    pub material_end_offsets: RoArray<'r, u32>, // relative to the start of the first offset

    // materials
    #[auto_struct(
            init = material_end_offsets.iter()
                .map(
                    |x| x - material_end_offsets.iter().nth(
                        material_end_offsets.iter().position(|y| y == x)
                        .unwrap().checked_sub(1)
                        .unwrap_or(usize::MAX)
                    ).unwrap_or(0)
                ).collect()
        )
    ]
    pub materials: IteratorArray<'r, CmdlMaterial<'r>, RoArrayIter<'r, u32>>,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct CmdlMaterial<'r>
{
    #[auto_struct(args)]
    size: u32,

    pub flags: u32,

    #[auto_struct(init = ((size - 4) as usize, ()))]
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
