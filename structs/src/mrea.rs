
use auto_struct_macros::auto_struct;
use reader_writer::{LCow, IteratorArray, Readable, Reader, RoArray, RoArrayIter, Writable};
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;

use std::io;

use crate::scly::Scly;


#[auto_struct(Readable, Writable)]
#[derive(Clone, Debug)]
pub struct Mrea<'r>
{
    #[auto_struct(expect = 0xDEADBEEF)]
    magic: u32,

    #[auto_struct(expect = 0xF)]
    version: u32,


    pub area_transform: GenericArray<f32, U12>,
    pub world_model_count: u32,

    #[auto_struct(derive = sections.len() as u32)]
    sections_count: u32,

    pub world_geometry_section_idx: u32,
    pub scly_section_idx: u32,
    pub collision_section_idx: u32,
    pub unknown_section_idx: u32,
    pub lights_section_idx: u32,
    pub visibility_tree_section_idx: u32,
    pub path_section_idx: u32,
    pub area_octree_section_idx: u32,

    #[auto_struct(derive_from_iter = sections.iter()
            .map(&|i: LCow<MreaSection>| i.size() as u32))]
    #[auto_struct(init = (sections_count as usize, ()))]
    section_sizes: RoArray<'r, u32>,

    #[auto_struct(pad_align = 32)]
    _pad: (),

    // TODO: A more efficient representation might be nice
    //       (We don't actually care about any of the sections except for scripting
    //        section, so we could treat them as raw bytes. Similarly the indicies
    //        for all the other sections.)

    #[auto_struct(init = section_sizes.iter())]
    pub sections: IteratorArray<'r, MreaSection<'r>, RoArrayIter<'r, u32>>,

    #[auto_struct(pad_align = 32)]
    _pad: (),
}


impl<'r> Mrea<'r>
{
    pub fn scly_section<'s>(&'s self) -> LCow<'s, Scly<'r>>
    {
        let section = self.sections.iter().nth(self.scly_section_idx as usize).unwrap();
        match section {
            LCow::Owned(MreaSection::Unknown(ref reader)) => LCow::Owned(reader.clone().read(())),
            LCow::Borrowed(MreaSection::Unknown(ref reader)) => LCow::Owned(reader.clone().read(())),
            LCow::Owned(MreaSection::Scly(scly)) => LCow::Owned(scly),
            LCow::Borrowed(MreaSection::Scly(scly)) => LCow::Borrowed(scly),
        }
    }

    pub fn scly_section_mut(&mut self) -> &mut Scly<'r>
    {
        self.sections.as_mut_vec()[self.scly_section_idx as usize].convert_to_scly()
    }
}

#[derive(Debug, Clone)]
pub enum MreaSection<'r>
{
    Unknown(Reader<'r>),
    Scly(Scly<'r>),
}

impl<'r> MreaSection<'r>
{
    // XXX A nicer/more clear name, maybe?
    pub fn convert_to_scly(&mut self) -> &mut Scly<'r>
    {
        *self = match *self {
            MreaSection::Unknown(ref reader) => MreaSection::Scly(reader.clone().read(())),
            MreaSection::Scly(ref mut scly) => return scly,
        };
        match *self {
            MreaSection::Scly(ref mut scly) => scly,
            _ => unreachable!(),
        }
    }
}

impl<'r> Readable<'r> for MreaSection<'r>
{
    type Args = u32;
    fn read_from(reader: &mut Reader<'r>, size: u32) -> Self
    {
        let res = MreaSection::Unknown(reader.truncated(size as usize));
        reader.advance(size as usize);
        res
    }

    fn size(&self) -> usize
    {
        match *self {
            MreaSection::Unknown(ref reader) => reader.len(),
            MreaSection::Scly(ref scly) => scly.size()
        }
    }
}

impl<'r> Writable for MreaSection<'r>
{
    fn write_to<W: io::Write>(&self, writer: &mut W) -> io::Result<u64>
    {
        match *self {
            MreaSection::Unknown(ref reader) => {
                writer.write_all(&reader)?;
                Ok(reader.len() as u64)
            },
            MreaSection::Scly(ref scly) => scly.write_to(writer),
        }
    }
}
