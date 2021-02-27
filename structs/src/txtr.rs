use auto_struct_macros::auto_struct;

use reader_writer::generic_array::GenericArray;
use reader_writer::generic_array::typenum::{U32, U512};
use reader_writer::{IteratorArray, LazyArray, RoArray, Reader};

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
    #[auto_struct(derive = format.txtr_format())]
    hdr_format: u32,

    pub width: u16,
    pub height: u16,
    #[auto_struct(derive = pixel_data.len() as u32)]
    mipmap_count: u32,

    // TODO Palettes...
    #[auto_struct(init = if has_palette(hdr_format) { Some(()) } else { None })]
    #[auto_struct(derive = format.palette())]
    palette: Option<TxtrPalette<'_>>,

    #[auto_struct(literal = TxtrFormat::new(hdr_format, &palette))]
    pub format: TxtrFormat,

    #[auto_struct(init = MipmapSizeIter::new(width, height, format.txtr_format(), mipmap_count))]
    pub pixel_data: IteratorArray<'r, LazyArray<'r, u8>, MipmapSizeIter>,

    // #[auto_struct(pad_align = 32)]
    // _pad: (),
}

#[derive(Clone, Debug)]
pub enum TxtrFormat
{
    I4,
    I8,
    Ia4,
    Ia8,
    C4(TxtrPaletteFormat, Box<GenericArray<u8, U32>>),
    C8(TxtrPaletteFormat, Box<GenericArray<u8, U512>>),
    Rgb565,
    Rgb5A3,
    Rgba8,
    Cmpr,
}

impl TxtrFormat
{
    fn new(fmt: u32, palette: &Option<TxtrPalette>) -> Self
    {
        match fmt {
            0x0 => TxtrFormat::I4,
            0x1 => TxtrFormat::I8,
            0x2 => TxtrFormat::Ia4,
            0x3 => TxtrFormat::Ia8,
            0x4 => {
                let palette = palette.as_ref().unwrap();
                TxtrFormat::C4(
                    TxtrPaletteFormat::from_u32(palette.format),
                    Box::new(palette.color_data.iter().collect()),
                )
            },
            0x5 => {
                let palette = palette.as_ref().unwrap();
                TxtrFormat::C8(
                    TxtrPaletteFormat::from_u32(palette.format),
                    Box::new(palette.color_data.iter().collect()),
                )
            },
            0x7 => TxtrFormat::Rgb565,
            0x8 => TxtrFormat::Rgb5A3,
            0x9 => TxtrFormat::Rgba8,
            0xa => TxtrFormat::Cmpr,
            fmt => panic!("Uknown or unsupported TXTR format: {:#x}", fmt),
        }
    }

    fn txtr_format(&self) -> u32
    {
        match self {
            TxtrFormat::I4 => 0x0,
            TxtrFormat::I8 => 0x1,
            TxtrFormat::Ia4 => 0x2,
            TxtrFormat::Ia8 => 0x3,
            TxtrFormat::C4(_, _) => 0x4,
            TxtrFormat::C8(_, _) => 0x5,
            TxtrFormat::Rgb565 => 0x7,
            TxtrFormat::Rgb5A3 => 0x8,
            TxtrFormat::Rgba8 => 0x9,
            TxtrFormat::Cmpr => 0xa,

        }
    }

    fn palette<'a>(&'a self) -> Option<TxtrPalette<'a>>
    {
        let (format, bytes, width, height) = match self {
            TxtrFormat::C4(fmt, bytes) => (fmt, &bytes[..], 1, 16),
            TxtrFormat::C8(fmt, bytes) => (fmt, &bytes[..], 256, 1),
            _ => return None,
        };
        Some(TxtrPalette {
            format: match format {
                TxtrPaletteFormat::Ia8 => 0x0,
                TxtrPaletteFormat::Rgb565 => 0x1,
                TxtrPaletteFormat::Rgb5A3 => 0x2,
            },
            width, height,
            color_data: Reader::new(bytes).read((bytes.len(), ())),
        })

    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TxtrPaletteFormat
{
    Ia8,
    Rgb565,
    Rgb5A3,
}

impl TxtrPaletteFormat
{
    fn from_u32(x: u32) -> Self
    {
        match x {
            0x0 => TxtrPaletteFormat::Ia8,
            0x1 => TxtrPaletteFormat::Rgb565,
            0x2 => TxtrPaletteFormat::Rgb5A3,
            _ => panic!("Invalid TXTR palette format {:#x}", x),
        }
    }
}


#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
struct TxtrPalette<'r>
{
    format: u32,
    width: u16,
    height: u16,

    #[auto_struct(init = (height as usize * width as usize * 2, ()))]
    color_data: RoArray<'r, u8>,
}

fn has_palette(format: u32) -> bool
{
    match format {
        0x4 | 0x5| 0x6 => true,
        _ => false,
    }
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
