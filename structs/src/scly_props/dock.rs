
use SclyPropertyData;
use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;

auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Debug, Clone)]
    pub struct Dock<'a>
    {
        #[expect = 7]
        prop_count: u32,

        name: CStr<'a>,

        unknown0: u8,
        position: GenericArray<f32, U3>,
        scale: GenericArray<f32, U3>,
        dock_number: f32,
        this_room: u8,
        unknown1: u8,
    }
}

impl<'a> SclyPropertyData for Dock<'a>
{
    fn object_type() -> u8
    {
        0x0B
    }
}
