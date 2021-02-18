use auto_struct_macros::auto_struct;

use reader_writer::{IteratorArray, LazyArray};

#[derive(Debug, Clone)]
pub struct MipmapSizeIter {
    width: usize,
    height: usize,
    format: u32,
    count: u32
}

impl MipmapSizeIter
{
    fn new(width: u16, height: u16, format: u32, count: u32) -> Self
    {
        MipmapSizeIter {
            width: width as usize,
            height: height as usize,
            format,
            count,
        }
    }
}

impl Iterator for MipmapSizeIter
{
    type Item = (usize, ());
    fn next(&mut self) -> Option<Self::Item>
    {
        if self.count == 0 {
            None
        } else {
            let ret = format_pixel_bytes(self.format, self.height * self.width);
            self.count -= 1;
            self.width /= 2;
            self.height /= 2;
            Some((ret, ()))
        }
    }
}

impl ExactSizeIterator for MipmapSizeIter
{
    fn len(&self) -> usize
    {
        self.count as usize
    }
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Txtr<'r>
{

    pub format: u32,

    pub width: u16,
    pub height: u16,
    pub mipmap_count: u32,

    // TODO Palettes...

    #[auto_struct(init = MipmapSizeIter::new(width, height, format, mipmap_count))]
    pub pixel_data: IteratorArray<'r, LazyArray<'r, u8>, MipmapSizeIter>,

    // #[auto_struct(pad_align = 32)]
    // _pad: (),
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
