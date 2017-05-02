
use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;

use scly_props::structs::DamageInfo;

auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Debug, Clone)]
    pub struct Trigger<'a>
    {
        // 9 properties
        name: CStr<'a>,

        position: GenericArray<f32, U3>,
        scale: GenericArray<f32, U3>,
        damage_info: DamageInfo,
        unknown0: GenericArray<f32, U3>,
        unknown1: u32,
        unknown2: u8,
        unknown3: u8,
        unknown4: u8,
    }
}
