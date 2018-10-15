
use SclyPropertyData;
use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;

auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Debug, Clone)]
    pub struct SpecialFunction<'a>
    {
        #[expect = 15]
        prop_count: u32,

        name: CStr<'a>,

        position: GenericArray<f32, U3>,
        rotation: GenericArray<f32, U3>,

        type_: u32,

        unknown0: CStr<'a>,
        unknown1: f32,
        unknown2: f32,
        unknown3: f32,

        layer_change_room_id: u32,
        layer_change_layer_id: u32,
        item_id: u32,

        unknown4: u8,
        unknown5: f32,

        // "Used by SpinnerController"
        unknown6: u32,
        unknown7: u32,
        unknown8: u32,
    }
}

impl<'a> SclyPropertyData for SpecialFunction<'a>
{
    const OBJECT_TYPE: u8 = 0x3A;
}
