
use SclyPropertyData;
use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use scly_props::structs::LightParameters;


auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Debug, Clone)]
    pub struct Effect<'a>
    {
        #[expect = 24]
        prop_count: u32,

        name: CStr<'a>,

        position: GenericArray<f32, U3>,
        rotation: GenericArray<f32, U3>,
        scale: GenericArray<f32, U3>,

        part: u32,
        elsc: u32,

        unknown0: u8,
        unknown1: u8,
        unknown2: u8,
        unknown3: u8,
        unknown4: u8,
        unknown5: f32,
        unknown6: f32,
        unknown7: f32,
        unknown8: f32,
        unknown9: u8,
        unknown10: f32,
        unknown11: f32,
        unknown12: f32,
        unknown13: u8,
        unknown14: u8,
        unknown15: u8,
        unknown16: u8,

        light_params: LightParameters,
    }
}

impl<'a> SclyPropertyData for Effect<'a>
{
    fn object_type() -> u8
    {
        0x7
    }
}
