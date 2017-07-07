
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use std::marker::PhantomData;

auto_struct! {
    #[auto_struct(Readable, Writable, FixedSize)]
    #[derive(Debug, Clone)]
    pub struct Scan<'a>
    {
        #[expect = 5]
        version: u32,
        #[expect = 0x0BADBEEF]
        magic: u32,

        frme: u32,
        strg: u32,

        scan_speed: u32,
        category: u32,
        icon_flag: u8,

        images: GenericArray<ScanImage, U4>,
        padding: GenericArray<u8, U23>,

        // Dummy so we can have a <'a>
        _dummy: PhantomData<&'a ()>,
    }
}

auto_struct! {
    #[auto_struct(Readable, Writable, FixedSize)]
    #[derive(Debug, Clone, Default)]
    pub struct ScanImage
    {
        txtr: u32,
        appearance_percent: f32,
        image_position: u32,
        width: u32,
        height: u32,
        interval: f32,
        fade_duration: f32,
    }
}

#[test]
fn test_scan_size()
{
    use reader_writer::Readable;
    assert_eq!(Scan::fixed_size().unwrap(), 0xA0);
}
