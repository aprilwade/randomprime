use auto_struct_macros::auto_struct;

use reader_writer::{Readable, Reader, RoArray};
use reader_writer::generic_array::{GenericArray, typenum:: *};

use crate::ResId;
use crate::res_id::*;

#[derive(Debug, Clone)]
pub enum Anim<'r>
{
    Uncompressed(AnimUncompressed<'r>),
    Compressed(AnimCompressed<'r>),
}


impl<'r> Readable<'r> for Anim<'r>
{
    type Args = ();
    fn read_from(reader: &mut Reader<'r>, (): ()) -> Self
    {
        let kind: u32 = reader.read(());
        let res = match kind {
            0 => Anim::Uncompressed(reader.read(())),
            2 => Anim::Compressed(reader.read(())),
            i => panic!("Invalid ANIM kind {}", i),
        };
        res
    }

    fn size(&self) -> usize
    {
        u32::fixed_size().unwrap() + match self {
            Anim::Uncompressed(ref i) => i.size(),
            Anim::Compressed(ref i) => i.size(),
        }
    }
}

/*
impl<'r> Writable for Anim<'r>
{
    fn write_to<W: io::Write>(&self, writer: &mut W) -> io::Result<u64>
    {
        Ok(match self {
            Anim::Uncompressed(anim) => 0u32.write_to(writer)? + anim.write_to(writer)?,
            Anim::Compressed(anim) => 2u32.write_to(writer)? + anim.write_to(writer)?,
        })
    }
}
*/

#[auto_struct(Readable)]
#[derive(Debug, Clone)]
pub struct AnimUncompressed<'r>
{
    duration: CharAnimTime,
    key_interval: CharAnimTime,
    key_count: u32,
    root_bone_id: u32,

    #[auto_struct(derive = bone_channel_index_array.len() as u32)]
    bone_channel_index_count: u32,
    #[auto_struct(init = (bone_channel_index_count as usize, ()))]
    bone_channel_index_array: RoArray<u8, 'r>,

    #[auto_struct(derive = translation_channel_index_array.len() as u32)]
    translation_channel_index_count: u32,
    #[auto_struct(init = (translation_channel_index_count as usize, ()))]
    translation_channel_index_array: RoArray<u8, 'r>,

    #[auto_struct(derive = rotation_key_array.len() as u32)]
    rotation_key_count: u32,
    #[auto_struct(init = (rotation_key_count as usize, ()))]
    rotation_key_array: RoArray<GenericArray<f32, U4>, 'r>,

    #[auto_struct(derive = translation_key_array.len() as u32)]
    translation_key_count: u32,
    #[auto_struct(init = (translation_key_count as usize, ()))]
    translation_key_array: RoArray<GenericArray<f32, U3>, 'r>,

    evnt: ResId<EVNT>,
}


#[auto_struct(Readable)]
#[derive(Debug, Clone)]
pub struct AnimCompressed<'r>
{
    scratch_size: u32,
    evnt: ResId<EVNT>,

    #[auto_struct(expect = 0x1)]
    unknown0: u32,

    duration: f32,
    interval: f32,
    root_bone_id: u32,
    looping_flag: u32,
    rotation_divisor: u32,
    translation_multiplier: f32,

    bone_channel_count: u32,

    #[auto_struct(expect = 0x1)]
    unknown1: u32,

    key_bitmap_length: u32,
    #[auto_struct(init = ((((key_bitmap_length + 31) & !31) / 32) as usize, ()))]
    key_bitmap_array: RoArray<u32, 'r>,

    bone_channel_count_2: u32,

    // #[auto_struct(derive = bone_channel_descriptor_array.len() as u32)]
    // bone_channel_descriptor_count: u32,
    // #[auto_struct(init = (bone_channel_descriptor_count as usize, ()))]
    // bone_channel_descriptor_array: RoArray<BoneChannelDescriptor, 'r>,
}

// #[auto_struct(Readable, FixedSize)]
// #[derive(Debug, Clone)]
// pub struct BoneChannelDescriptor
// {
//     bone_id: u32,
//     rotation_key_count: u16,
//     intial_rotation_x: i16,
//     rotation_bits_x: u8,
//     intial_rotation_y: i16,
//     rotation_bits_y: u8,
//     intial_rotation_z: i16,
//     rotation_bits_z: u8,

//     translation_key_count: u16,
//     intial_translation_x: i16,
//     translation_bits_x: u8,
//     intial_translation_y: i16,
//     translation_bits_y: u8,
//     intial_translation_z: i16,
//     translation_bits_z: u8,
// }

#[auto_struct(Readable, FixedSize)]
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CharAnimTime
{
    time: f32,
    differential_state: u32,
}
