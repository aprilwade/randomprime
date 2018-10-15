
use SclyPropertyData;
use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use scly_props::structs::ScannableParameters;

auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Debug, Clone)]
    pub struct PointOfInterest<'a>
    {
        #[expect = 6]
        prop_count: u32,

        name: CStr<'a>,

        position: GenericArray<f32, U3>,
        rotation: GenericArray<f32, U3>,
        unknown0: u8,
        scan_param: ScannableParameters,
        unknown1: f32,
    }
}

impl<'a> SclyPropertyData for PointOfInterest<'a>
{
    const OBJECT_TYPE: u8 = 0x42;
}
