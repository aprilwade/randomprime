use auto_struct_macros::auto_struct;

use reader_writer::{/* IteratorArray,*/ LazyArray};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Txtr<'r>
{

    pub format: u32,

    pub width: u16,
    pub height: u16,
    pub mipmap_count: u32,

    // TODO Palettes...

    #[auto_struct(init = (format_pixel_bytes(format, (height * width) as usize), ()))]
    pub pixel_data: LazyArray<'r, u8>,
    // TODO: Mipmaps

}

fn format_pixel_bytes(format: u32, pixels: usize) -> usize
{
    match format {
        0x0 => pixels / 2,
        0x1 => pixels,
        0x2 => pixels,
        0x3 => pixels * 2,
        0x4 => pixels / 2,
        0x5 => pixels,
        0x6 => pixels * 2,
        0x7 => pixels * 2,
        0x8 => pixels * 2,
        0x9 => pixels * 4,
        0xA => pixels / 2,
        _ => panic!(),
    }
}
