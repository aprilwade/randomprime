
use reader_writer::{FourCC, FixedArray, RoArray};
use reader_writer::typenum::{U4096, U2048, U32, U64, U128, Sum};

auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Debug, Clone)]
    pub struct Bnr<'a>
    {
        #[expect = b"BNR1".into()]
        magic: FourCC,

        padding: RoArray<'a, u8> = (0x1c, ()),

        pixels: FixedArray<u8, Sum<U4096, U2048>>,// 0x1800

        game_name: FixedArray<u8, U32>,// 0x20
        developer: FixedArray<u8, U32>,// 0x20

        game_name_full: FixedArray<u8, U64>,// 0x40
        developer_full: FixedArray<u8, U64>,// 0x40
        description: FixedArray<u8, U128>,// 0x80
    }
}
