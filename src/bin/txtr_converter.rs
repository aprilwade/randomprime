#[macro_use]
extern crate clap;

use image::{ColorType, ImageDecoder};
use image::codecs::png::{PngDecoder, PngEncoder};

use libsquish_wrapper::{compress_dxt1gcn_block, decompress_dxt1gcn_block};
use reader_writer::{Reader, Writable};

use std::convert::{TryFrom, TryInto};
use std::fs::File;
use std::path::Path;

fn txtr2png(input: &Path, output: &Path, mipmap: usize) -> Result<(), String> {
    let input_file = File::open(input)
        .map_err(|e| format!("Failed to open input file: {}", e))?;
    let output_file = File::create(output)
        .map_err(|e| format!("Failed to open output file: {}", e))?;
    let mmap = unsafe { memmap::Mmap::map(&input_file) }
        .map_err(|e| format!("Failed to map input file: {}", e))?;
    let mut reader = Reader::new(&mmap[..]);

    // TODO: Catch a potential panic here
    let mut txtr: structs::Txtr = reader.read(());

    if mipmap > txtr.pixel_data.len() {
        Err(format!("TXTR only contains {} mipmaps", txtr.pixel_data.len()))?;
    }

    let w = txtr.width as usize >> mipmap;
    let h = txtr.height as usize >> mipmap;

    let mipmap_data = &txtr.pixel_data.as_mut_vec()[mipmap].as_mut_vec()[..];

    let format = match txtr.format {
        0x0 => Format::I4,
        0x1 => Format::I8,
        0x2 => Format::Ia4,
        0x3 => Format::Ia8,
        0x7 => Format::Rgb565,
        0x8 => Format::Rgb5A3,
        0x9 => Format::Rgba8,
        0xa => Format::Cmpr,
        _ => panic!("Unsupported TXTR format {:#x}", txtr.format),
    };
    let color_type: ColorType = format.into();
    let channel_count = color_type.channel_count() as usize;
    let mut decompressed_pixels = vec![0u8; w * h * channel_count];

    let (block_w, block_h) = format.block_dimensions();

    let mut decoded_block = vec![0u8; channel_count * block_w * block_h];
    for (i, block) in mipmap_data.chunks(format.bytes_per_block()).enumerate() {
        format.decode_block(block, &mut decoded_block[..]);
        let outer_x = (i % (w / block_w)) * block_w;
        let outer_y = (i / (w / block_w)) * block_w;
        for inner_y in 0..block_h {
            for inner_x in 0..block_w {
                let decoded_start = (inner_y * block_w + inner_x) * channel_count;
                let pixels_start = ((h - 1 - (outer_y + inner_y)) * w + outer_x + inner_x)
                    * channel_count;
                decompressed_pixels[pixels_start..pixels_start + channel_count]
                    .copy_from_slice(&decoded_block[decoded_start..decoded_start + channel_count]);
            }
        }
    }

    let encoder = PngEncoder::new(output_file);
    encoder.encode(
        &decompressed_pixels[..],
        w as u32,
        h as u32,
        color_type,
    ).map_err(|e| format!("Failed to encode PNG: {}", e))?;

    Ok(())
}

fn png2txtr(input: &Path, output: &Path, format: Format) -> Result<(), String> {
    let input_file = File::open(input)
        .map_err(|e| format!("Failed to open input file: {}", e))?;
    let output_file = File::create(output)
        .map_err(|e| format!("Failed to open output file: {}", e))?;

    let decoder = PngDecoder::new(input_file)
        .map_err(|e| format!("Failed to decode input file as PNG: {}", e))?;
    let color_type: ColorType = format.into();
    if decoder.color_type() != color_type {
        panic!("Incorrect PNG type {:?} for TXTR format {:?}", decoder.color_type(), format);
    }
    let (w, h) = decoder.dimensions();
    let (w, h) = (w as usize, h as usize);
    let (block_w, block_h) = format.block_dimensions();
    if w % block_w != 0 || h % block_h != 0 {
        panic!("The images width and height ({}, {}) must be a multiple of the chosen format's block dimensions ({}, {})", w, h, block_w, block_h);
    }

    let channel_count = color_type.channel_count() as usize;

    let mut uncompressed_pixels = vec![0u8; decoder.total_bytes() as usize];
    decoder.read_image(&mut uncompressed_pixels[..])
        .map_err(|e| format!("Error reading input PNG: {}", e))?;

    let mut mipmaps = vec![];

    let mut mipmap_count = 0;
    // Temporary buffer to store the pixels for 1 block so we can pass it to the decode function
    let mut block_pixels = vec![0u8; block_w * block_h * channel_count];
    for _ in 0.. {
        if (w >> mipmap_count) % block_w != 0 || (h >> mipmap_count) % block_h != 0 {
            break
        }
        let w = w >> mipmap_count;
        let h = h >> mipmap_count;
        mipmap_count += 1;

        let mut blocks = vec![0u8; w * h * format.bytes_per_block() / (block_w * block_h)];

        for (i, block) in blocks.chunks_mut(format.bytes_per_block()).enumerate() {
            let outer_x = (i % (w / block_w)) * block_w;
            let outer_y = (i / (w / block_w)) * block_w;
            for inner_y in 0..block_h {
                for inner_x in 0..block_w {
                    let block_start = (inner_y * block_w + inner_x) * channel_count;
                    let image_start = ((h - 1 - (outer_y + inner_y)) * w + outer_x + inner_x)
                        * channel_count;
                    block_pixels[block_start..block_start + channel_count]
                        .copy_from_slice(&uncompressed_pixels[image_start..image_start + channel_count]);
                }
            }
            format.encode_block(block, &block_pixels[..]);
        }

        uncompressed_pixels = box_filter_pixels(
            &uncompressed_pixels[..],
            w,
            h,
            channel_count,
            format == Format::Cmpr,
        );

        mipmaps.push(blocks.into());
    }

    let txtr = structs::Txtr {
        format: format.into(),

        width: w as u16,
        height: h as u16,
        mipmap_count: mipmap_count as u32,

        pixel_data: mipmaps.into(),
    };

    txtr.write_to(&mut &output_file)
        .map_err(|e| format!("Error writing TXTR: {}", e))?;

    Ok(())
}

fn box_filter_pixels(pixels: &[u8], w: usize, h: usize, chan_count: usize, discretize_alpha: bool)
    -> Vec<u8>
{
    let mut output = Vec::with_capacity(w * h * chan_count / 4);
    for iy in 0..h / 2 {
        for ix in 0..w / 2 {
            let y = iy * 2;
            let x = ix * 2;
            for c in 0..chan_count {
                output.push(((
                        (pixels[(y * w + x) * chan_count + c] as u16)
                        + (pixels[(y * w + x + 1) * chan_count + c] as u16)
                        + (pixels[((y + 1) * w + x ) * chan_count + c] as u16)
                        + (pixels[((y + 1) * w + x + 1) * chan_count + c] as u16)
                    ) / 4) as u8
                );
                if discretize_alpha && c == chan_count - 1 {
                    let last = output.last_mut().unwrap();
                    if *last > 0 {
                        *last = 0xff;
                    }
                }
            }
        }
    }
    output
}


#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Format {
    I4,
    I8,
    Ia4,
    Ia8,
    Rgb565,
    Rgb5A3,
    Rgba8,
    Cmpr,
}

// XXX The following conversion functions are borrowed from URDE
// https://github.com/AxioDL/urde/blob/master/DataSpec/DNACommon/TXTR.cpp
fn convert3to8(v: u8) -> u8 {
    (v << 5) | (v << 2)| (v >> 1)
}

fn convert4to8(v: u8) -> u8 {
    (v << 4) | v
}

fn convert5to8(v: u8) -> u8 {
    (v << 3) | (v >> 2)
}

fn convert6to8(v: u8) -> u8 {
    (v << 2) | (v >> 4)
}

impl Format {
    fn bytes_per_block(self) -> usize {
        match self {
            Format::I4 => 32,
            Format::I8 => 32,
            Format::Ia4 => 32,
            Format::Ia8 => 32,
            Format::Rgb565 => 32,
            Format::Rgb5A3 => 32,
            Format::Rgba8 => 64,
            Format::Cmpr => 32,
        }
    }

    fn block_dimensions(self) -> (usize, usize) {
        match self {
            Format::I4 => (8, 8),
            Format::I8 => (8, 4),
            Format::Ia4 => (8, 4),
            Format::Ia8 => (4, 4),
            Format::Rgb565 => (4, 4),
            Format::Rgb5A3 => (4, 4),
            Format::Rgba8 => (4, 4),
            Format::Cmpr => (8, 8),
        }
    }

    fn decode_block(self, block: &[u8], pixels: &mut[u8])
    {
        assert_eq!(block.len(), self.bytes_per_block());
        let channel_count = <Format as Into<ColorType>>::into(self).channel_count() as usize;
        assert_eq!(
            pixels.len(),
            channel_count * self.block_dimensions().0 * self.block_dimensions().1,
        );
        match self {
            Format::I4 => {
                for (block_byte, pixel_bytes) in block.iter().zip(pixels.chunks_mut(2)) {
                    pixel_bytes[0] = convert4to8(block_byte & 0xf);
                    pixel_bytes[1] = convert4to8(block_byte >> 4);
                }
            },
            Format::I8 => pixels.copy_from_slice(block),
            Format::Ia4 => {
                for (block_byte, pixel_bytes) in block.iter().zip(pixels.chunks_mut(2)) {
                    pixel_bytes[0] = convert4to8(block_byte & 0xf);
                    pixel_bytes[1] = convert4to8(block_byte >> 4);
                }
            },
            Format::Ia8 => pixels.copy_from_slice(block),
            Format::Rgb565 => {
                for (block_bytes, pixel_bytes) in block.chunks(2).zip(pixels.chunks_mut(3)) {
                    let v = u16::from_be_bytes(block_bytes.try_into().unwrap());
                    pixel_bytes[0] = convert5to8(((v >> 11) & 0x1f) as u8);
                    pixel_bytes[1] = convert6to8(((v >> 5) & 0x3f) as u8);
                    pixel_bytes[2] = convert5to8((v & 0x1f) as u8);
                }
            },
            Format::Rgb5A3 => {
                for (block_bytes, pixel_bytes) in block.chunks(2).zip(pixels.chunks_mut(4)) {
                    let v = u16::from_be_bytes(block_bytes.try_into().unwrap());
                    if v & 0x8000 != 0 {
                        pixel_bytes[0] = convert5to8(((v >> 10) & 0x1f) as u8);
                        pixel_bytes[1] = convert5to8(((v >> 5) & 0x1f) as u8);
                        pixel_bytes[2] = convert5to8((v & 0x1f) as u8);
                        pixel_bytes[3] = 0xff;
                    } else {
                        pixel_bytes[0] = convert4to8(((v >> 8) & 0xf) as u8);
                        pixel_bytes[1] = convert4to8(((v >> 4) & 0xf) as u8);
                        pixel_bytes[2] = convert4to8((v & 0xf) as u8);
                        pixel_bytes[3] = convert3to8((v >> 12 & 0x7) as u8);
                    }
                }
            },
            Format::Rgba8 => pixels.copy_from_slice(block),
            Format::Cmpr => {
                let mut decoded_dxt1_block = [[0u8; 4]; 16];
                for i in 0..4 {
                    decompress_dxt1gcn_block(
                        &mut decoded_dxt1_block,
                        block[i * 8..(i + 1) * 8].try_into().unwrap(),
                    );

                    let outer_x = i % 2 * 4;
                    let outer_y = i / 2 * 4;
                    for (k, decoded_pixel) in decoded_dxt1_block.iter().enumerate() {
                        let inner_x = k % 4;
                        let inner_y = k / 4;
                        let start = (outer_y + inner_y) * 32 + (outer_x + inner_x) * 4;
                        pixels[start..start + 4].copy_from_slice(&decoded_pixel[..]);
                    }
                }
            },
        }
    }

    fn encode_block(self, block: &mut [u8], pixels: &[u8])
    {
        assert_eq!(block.len(), self.bytes_per_block());
        let channel_count = <Format as Into<ColorType>>::into(self).channel_count() as usize;
        assert_eq!(
            pixels.len(),
            channel_count * self.block_dimensions().0 * self.block_dimensions().1,
        );
        match self {
            Format::I4 => {
                for (block_byte, pixel_bytes) in block.iter_mut().zip(pixels.chunks(2)) {
                    *block_byte = (pixel_bytes[0] >> 4) | ((pixel_bytes[1] >> 4) << 4);
                }
            },
            Format::I8 => block.copy_from_slice(pixels),
            Format::Ia4 => {
                for (block_byte, pixel_bytes) in block.iter_mut().zip(pixels.chunks(2)) {
                    *block_byte = (pixel_bytes[0] >> 4) | ((pixel_bytes[1] >> 4) << 4);
                }
            },
            Format::Ia8 => block.copy_from_slice(pixels),
            Format::Rgb565 => {
                for (block_bytes, pixel_bytes) in block.chunks_mut(2).zip(pixels.chunks(3)) {
                    let v = ((pixel_bytes[0] as u16 >> 3) << 11)
                            | ((pixel_bytes[1] as u16 >> 2) << 5)
                            | ((pixel_bytes[2] as u16 >> 3));
                    block_bytes.copy_from_slice(&v.to_be_bytes()[..]);
                }
            },
            Format::Rgb5A3 => {
                for (block_bytes, pixel_bytes) in block.chunks_mut(2).zip(pixels.chunks(4)) {
                    let v = if pixel_bytes[3] == 0xff {
                        0x8000 | ((pixel_bytes[0] as u16 >> 3) << 10)
                            | ((pixel_bytes[1] as u16 >> 3) << 5)
                            | (pixel_bytes[2] as u16 >> 3)
                    } else {
                        ((pixel_bytes[0] as u16 >> 4) << 8)
                            | ((pixel_bytes[1] as u16 >> 4) << 4)
                            | (pixel_bytes[2] as u16 >> 4)
                            | ((pixel_bytes[3] as u16 >> 5) << 12)
                    };
                    block_bytes.copy_from_slice(&v.to_be_bytes()[..]);
                }
            },
            Format::Rgba8 => block.copy_from_slice(pixels),
            Format::Cmpr => {
                let mut sub_block_pixels = [[0u8; 4]; 16];
                for (i, sub_block) in block.chunks_mut(8).enumerate()  {
                    let outer_x = i % 2 * 4;
                    let outer_y = i / 2 * 4;
                    for (k, sub_block_pixel) in sub_block_pixels.iter_mut().enumerate() {
                        let inner_x = k % 4;
                        let inner_y = k / 4;
                        let start = (outer_y + inner_y) * 32 + (outer_x + inner_x) * 4;
                        sub_block_pixel[..].copy_from_slice(&pixels[start..start + 4]);
                    }

                    compress_dxt1gcn_block(
                        &sub_block_pixels,
                        sub_block.try_into().unwrap(),
                    );

                }
            },
        }
    }
}

impl Into<u32> for Format {
    fn into(self) -> u32 {
        match self {
            Format::I4 => 0x0,
            Format::I8 => 0x1,
            Format::Ia4 => 0x2,
            Format::Ia8 => 0x3,
            Format::Rgb565 => 0x7,
            Format::Rgb5A3 => 0x8,
            Format::Rgba8 => 0x9,
            Format::Cmpr => 0xa,
        }
    }
}

impl Into<ColorType> for Format {
    fn into(self) -> ColorType {
        match self {
            Format::I4 => ColorType::L8,
            Format::I8 => ColorType::L8,
            Format::Ia4 => ColorType::La8,
            Format::Ia8 => ColorType::La8,
            Format::Rgb565 => ColorType::Rgb8,
            Format::Rgb5A3 => ColorType::Rgba8,
            Format::Rgba8 => ColorType::Rgba8,
            Format::Cmpr => ColorType::Rgba8,
        }
    }

}

impl<'a> TryFrom<&'a str> for Format {
    type Error = ();
    fn try_from(s: &'a str) -> Result<Format, Self::Error> {
        match s.to_ascii_lowercase().as_str() {
            "i4" => Ok(Format::I4),
            "i8" => Ok(Format::I8),
            "ia4" => Ok(Format::Ia4),
            "ia8" => Ok(Format::Ia8),
            "rgb565" => Ok(Format::Rgb565),
            "rgb5a3" => Ok(Format::Rgb5A3),
            "rgba8" => Ok(Format::Rgba8),
            "cmpr" => Ok(Format::Cmpr),
            _ => Err(()),
        }
    }
}

fn main() {
    let app = clap_app!(app =>
        (version: crate_version!())
        (author: crate_authors!())
        (about: "Converts TXTRs to/from PNGs.")
        (@setting ArgRequiredElseHelp)
        (@subcommand txtr2png =>
            (about: "Converts a TXTR to a PNG.")
            (@arg input: -i --input +takes_value +required "Input TXTR file to convert.")
            (@arg output: -o --output +takes_value +required "Output path to write the PNG file.")
            (@arg mipmap: -m --mipmap +takes_value
                {|s| s.parse::<u8>().map(|_| ()).map_err(|_| "Expected integer for mipmap".into()) }
                "Which mipmap to extract. Defaults to 0"
            )
        )
        (@subcommand png2txtr =>
            (about: "Converts a PNG to a TXTR.")
            (@arg input: -i --input +takes_value +required "Input PNG file to convert.")
            (@arg output: -o --output +takes_value +required "Output path to write the TXTR file.")
            (@arg format: -f --format +takes_value +required
                {
                    |s| Format::try_from(s.as_str())
                        .map(|_| ())
                        .map_err(|()| format!("Unknown format \"{}\"", s))
                }
                "TXTR format to use. Excepted values are: I4, I8, IA4, IA8, RGB565, RGB5A3, RGBA8, CMPR (case-insensitive)"
            )
        )
    );
    let matches = app.get_matches();

    let res = match matches.subcommand() {
        ("txtr2png", Some(matches)) => txtr2png(
            matches.value_of("input").unwrap().as_ref(),
            matches.value_of("output").unwrap().as_ref(),
            matches.value_of("mipmap").unwrap_or("0").parse::<usize>().unwrap(),
        ),
        ("png2txtr", Some(matches)) => png2txtr(
            matches.value_of("input").unwrap().as_ref(),
            matches.value_of("output").unwrap().as_ref(),
            matches.value_of("format").unwrap().try_into().unwrap(),
        ),
        _ => return,
    };
    if let Err(s) = res {
        eprintln!("{} {}", clap::Format::Error("error:"), s);
    }
}
