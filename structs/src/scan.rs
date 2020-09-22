use auto_struct_macros::auto_struct;

use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;

use crate::ResId;
use crate::res_id::*;

use std::marker::PhantomData;

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct Scan<'r>
{
    #[auto_struct(expect = 5)]
    version: u32,
    #[auto_struct(expect = 0x0BADBEEF)]
    magic: u32,

    pub frme: ResId<FRME>,
    pub strg: ResId<STRG>,

    pub scan_speed: u32,
    pub category: u32,
    pub icon_flag: u8,

    pub images: GenericArray<ScanImage, U4>,
    pub padding: GenericArray<u8, U23>,

    // Dummy so we can have a <'r>
    pub _dummy: PhantomData<&'r ()>,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct ScanImage
{
    pub txtr: ResId<TXTR>,
    pub appearance_percent: f32,
    pub image_position: u32,
    pub width: u32,
    pub height: u32,
    pub interval: f32,
    pub fade_duration: f32,
}

impl Default for ScanImage
{
    fn default() -> Self
    {
        ScanImage {
            txtr: ResId::invalid(),
            appearance_percent: 0.0,
            image_position: 0xFFFFFFFF,
            width: 0,
            height: 0,
            interval: 0.0,
            fade_duration: 0.0,
        }
    }
}

#[test]
fn test_scan_size()
{
    use reader_writer::Readable;
    assert_eq!(Scan::fixed_size().unwrap(), 0xA0);
}
