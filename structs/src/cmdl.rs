
use reader_writer::{RoArray, RoArrayIter, IteratorArray};
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;

// We don't need to modify CMDLs, so most of the details are left out.
// We only actually care about reading out the TXTR file ids.
auto_struct! {
    #[auto_struct(Readable)]
    #[derive(Debug, Clone)]
    pub struct Cmdl<'a>
    {
        #[expect = 0xDEADBABE]
        magic: u32,

        #[expect = 2]
        version: u32,

        flags: u32,

        maab: GenericArray<f32, U6>,

        data_section_count: u32,
        material_set_count: u32,

        material_set_sizes: RoArray<'a, u32> = (material_set_count as usize, ()),
        data_section_sizes: RoArray<'a, u32> =
            ((data_section_count - material_set_count) as usize, ()),

        alignment_padding!(32),

        material_sets: IteratorArray<'a, MaterialSet<'a>, RoArrayIter<'a, u32>> =
            material_set_sizes.iter(),
    }
}

auto_struct! {
    #[auto_struct(Readable)]
    #[derive(Debug, Clone)]
    pub struct MaterialSet<'a>
    {
        #[args]
        size: u32,

        texture_count: u32,
        texture_ids: RoArray<'a, u32> = (texture_count as usize, ()),

        padding: RoArray<'a, u8> = (size as usize - 4 - texture_ids.size(), ()),
    }
}
