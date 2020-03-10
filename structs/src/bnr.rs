use auto_struct_macros::auto_struct;

use reader_writer::{FourCC, FixedArray, RoArray};
use reader_writer::typenum::{U4096, U2048, U32, U64, U128, U5, Sum};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Bnr<'r>
{
    #[auto_struct(derive = if other_lang_fields.is_some() { b"BNR2".into() } else { b"BNR1".into() })]
    magic: FourCC,

    #[auto_struct(init = (0x1c, ()))]
    pub padding: RoArray<'r, u8>,

    pub pixels: FixedArray<u8, Sum<U4096, U2048>>,// 0x1800

    pub english_fields: BnrMetadata,
    #[auto_struct(init = if magic == b"BNR2".into() { Some(()) } else { None })]
    pub other_lang_fields: Option<FixedArray<BnrMetadata, U5>>,
}


#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct BnrMetadata
{
    pub game_name: FixedArray<u8, U32>,// 0x20
    pub developer: FixedArray<u8, U32>,// 0x20

    pub game_name_full: FixedArray<u8, U64>,// 0x40
    pub developer_full: FixedArray<u8, U64>,// 0x40
    pub description: FixedArray<u8, U128>,// 0x80
}
