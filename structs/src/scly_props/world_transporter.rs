
use SclyPropertyData;
use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use scly_props::structs::AncsProp;

auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Debug, Clone)]
    pub struct WorldTransporter<'a>
    {
        #[expect = 21]
        prop_count: u32,

        name: CStr<'a>,

        unknown0: u8,
        mlvl: u32,
        mrea: u32,
        ancs: AncsProp,
        unknown1: GenericArray<f32, U3>,
        cmdl0: u32,
        unknown2: GenericArray<f32, U3>,
        cmdl1: u32,
        unknown3: GenericArray<f32, U3>,
        unknown4: u8,
        unknown5: u32,
        unknown6: u32,
        unknown7: u32,
        unknown8: u8,
        font: u32,
        strg: u32,
        unknown9: u8,
        unknown10: f32,
        unknown11: f32,
        unknown12: f32,
    }
}

impl<'a> SclyPropertyData for WorldTransporter<'a>
{
    fn object_type() -> u8
    {
        0x062
    }
}
