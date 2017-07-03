
use SclyPropertyData;
use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;

auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Debug, Clone)]
    pub struct Sound<'a>
    {
        #[expect = 20]
        prop_count: u32,

        name: CStr<'a>,

        position: GenericArray<f32, U3>,
        rotation: GenericArray<f32, U3>,

        // 17 unknown properties
        unknowns: GenericArray<u8, U44>,
    }
}

impl<'a> SclyPropertyData for Sound<'a>
{
    fn object_type() -> u8
    {
        0x9
    }
}
