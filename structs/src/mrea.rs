
use reader_writer::{Dap, ImmCow, IteratorArray, Readable, Reader, RoArray, RoArrayIter, Writable,
                    pad_bytes_count, pad_bytes};
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;

use std::io::Write;

use scly::Scly;


auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Clone, Debug)]
    pub struct Mrea<'a>
    {
        #[expect = 0xDEADBEEF]
        magic: u32,

        #[expect = 0xF]
        version: u32,


        //area_transform: Lazy<'a, Box<GenericArray<f, U12>>>,
        area_transform: GenericArray<f32, U12>,
        world_model_count: u32,

        #[derivable = sections.len() as u32]
        sections_count: u32,

        world_geometry_section_idx: u32,
        scly_section_idx: u32,
        collision_section_idx: u32,
        unknown_section_idx: u32,
        lights_section_idx: u32,
        visibility_tree_section_idx: u32,
        path_section_idx: u32,
        area_octree_section_idx: u32,

        #[derivable: Dap<_, _> = sections.iter()
                                          .map(&|i: ImmCow<MreaSection>| i.size() as u32).into()]
        section_sizes: RoArray<'a, u32> = (sections_count as usize, ()),

        #[offset]
        offset: usize,
        #[derivable = pad_bytes(32, offset)]
        _padding: RoArray<'a, u8> = (pad_bytes_count(32, offset), ()),

        // TODO: A more efficient representation would be nice
        //       (We don't actually care about any of the sections except for scripting
        //        section, so we could treat them as raw bytes. Similarly the indicies
        //        for all the other sections.)
        sections: IteratorArray<'a, MreaSection<'a>, RoArrayIter<'a, u32>> = section_sizes.iter(),
    }
}


/*
auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Debug, Clone)]
    pub struct MreaSection<'a>
    {
        #[args]
        args: u32,
        data: Array<'a, u8> = (args as usize, ()),
    }
}
*/

#[derive(Debug, Clone)]
pub enum MreaSection<'a>
{
    Unknown(Reader<'a>),
    Scly(Scly<'a>),
}

impl<'a> Readable<'a> for MreaSection<'a>
{
    type Args = u32;
    fn read(reader: Reader<'a>, size: u32) -> (Self, Reader<'a>)
    {
        (MreaSection::Unknown(reader.truncated(size as usize)), reader.offset(size as usize))
    }

    fn size(&self) -> usize
    {
        match *self {
            MreaSection::Unknown(ref reader) => reader.len(),
            MreaSection::Scly(ref scly) => scly.size()
        }
    }
}

impl<'a> Writable for MreaSection<'a>
{
    fn write<W: Write>(&self, writer: &mut W)
    {
        match *self {
            MreaSection::Unknown(ref reader) => writer.write_all(&reader).unwrap(),
            MreaSection::Scly(ref scly) => scly.write(writer),
        }
    }
}

// TODO: Implement a way to fetch the offset...
