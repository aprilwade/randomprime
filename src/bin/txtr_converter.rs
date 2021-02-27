#[macro_use]
extern crate clap;

use image::{ColorType, ImageDecoder};
use image::codecs::png::{PngDecoder, PngEncoder};

use structs::{Txtr, TxtrFormat, TxtrPaletteFormat};
use libsquish_wrapper::{compress_dxt1gcn_block, decompress_dxt1gcn_block};
use reader_writer::{Readable, Reader, Writable};

use std::convert::TryInto;
use std::collections::HashMap;
use std::fs::File;
use std::iter;
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
    let mut txtr: Txtr = reader.read(());

    if mipmap > txtr.pixel_data.len() {
        Err(format!("TXTR only contains {} mipmaps", txtr.pixel_data.len()))?;
    }

    let w = txtr.width as usize >> mipmap;
    let h = txtr.height as usize >> mipmap;

    let mipmap_data = &txtr.pixel_data.as_mut_vec()[mipmap].as_mut_vec()[..];

    let color_type = txtr.format.color_type();
    let channel_count = color_type.channel_count() as usize;
    let mut decompressed_pixels = vec![0u8; w * h * channel_count];

    let (block_w, block_h) = txtr.format.block_dimensions();

    let mut decoded_block = vec![0u8; channel_count * block_w * block_h];
    for (i, block) in mipmap_data.chunks(txtr.format.bytes_per_block()).enumerate() {
        txtr.format.decode_block(block, &mut decoded_block[..]);
        let outer_x = (i % (w / block_w)) * block_w;
        let outer_y = (i / (w / block_w)) * block_h;
        for inner_y in 0..block_h {
            for inner_x in 0..block_w {
                let decoded_start = (inner_y * block_w + inner_x) * channel_count;
                let pixels_start = if txtr.format.flipped() {
                    ((h - 1 - (outer_y + inner_y)) * w + outer_x + inner_x) * channel_count
                } else {
                    ((outer_y + inner_y) * w + outer_x + inner_x) * channel_count
                };
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

fn png2txtr(input: &Path, output: &Path, mipmap_count: Option<u8>, mut format: TxtrFormat)
    -> Result<(), String>
{
    let input_file = File::open(input)
        .map_err(|e| format!("Failed to open input file: {}", e))?;
    let output_file = File::create(output)
        .map_err(|e| format!("Failed to open output file: {}", e))?;

    let decoder = PngDecoder::new(input_file)
        .map_err(|e| format!("Failed to decode input file as PNG: {}", e))?;
    let color_type: ColorType = format.color_type();
    if decoder.color_type() != color_type {
        Err(format!("Incorrect PNG type {:?} for TXTR format {:?}", decoder.color_type(), format))?
    }
    let (w, h) = decoder.dimensions();
    let (w, h) = (w as usize, h as usize);
    let (block_w, block_h) = format.block_dimensions();
    if w % block_w != 0 || h % block_h != 0 {
        Err(format!(
            "The images width and height ({}, {}) must be a multiple of the chosen format's \
             block dimensions ({}, {})",
            w, h, block_w, block_h
        ))?
    }

    let channel_count = color_type.channel_count() as usize;

    let mut uncompressed_pixels = vec![0u8; decoder.total_bytes() as usize];
    decoder.read_image(&mut uncompressed_pixels[..])
        .map_err(|e| format!("Error reading input PNG: {}", e))?;

    format.compute_palette(&uncompressed_pixels[..])
        .map_err(|()| "Image contains too many colors for choosen paletted format")?;

    let max_mipmaps_for_fomat = match format {
        TxtrFormat::C4(_, _) | TxtrFormat::C8(_, _) => 1,
        _ => {
            let mut i = 1;
            while (w >> i) % block_w == 0 && (h >> i) % block_h == 0 {
                i += 1
            }
            i
        },
    };
    let max_mipmaps = mipmap_count.unwrap_or(max_mipmaps_for_fomat);
    if max_mipmaps > max_mipmaps_for_fomat {
        Err(format!("Specified format supports a max of {} mipmaps for this image",
                    max_mipmaps_for_fomat))?
    }

    let mut mipmaps = vec![];

    // Temporary buffer to store the pixels for 1 block so we can pass it to the decode function
    let mut block_pixels = vec![0u8; block_w * block_h * channel_count];
    for mipmap_count in 0..max_mipmaps {
        let w = w >> mipmap_count;
        let h = h >> mipmap_count;

        let mut blocks = vec![0u8; w * h * format.bytes_per_block() / (block_w * block_h)];

        for (i, block) in blocks.chunks_mut(format.bytes_per_block()).enumerate() {
            let outer_x = (i % (w / block_w)) * block_w;
            let outer_y = (i / (w / block_w)) * block_h;
            for inner_y in 0..block_h {
                for inner_x in 0..block_w {
                    let block_start = (inner_y * block_w + inner_x) * channel_count;
                    let image_start = if format.flipped() {
                        ((h - 1 - (outer_y + inner_y)) * w + outer_x + inner_x) * channel_count
                    } else {
                        ((outer_y + inner_y) * w + outer_x + inner_x) * channel_count
                    };
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
            matches!(format, TxtrFormat::Cmpr),
        );

        mipmaps.push(blocks.into());
    }

    let txtr = Txtr {
        format,

        width: w as u16,
        height: h as u16,

        pixel_data: mipmaps.into(),
    };

    txtr.write_to(&mut &output_file)
        .map_err(|e| format!("Error writing TXTR: {}", e))?;
    reader_writer::padding::pad_bytes(32, txtr.size()).write_to(&mut &output_file)
        .map_err(|e| format!("Error writing padding: {}", e))?;

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

// XXX The following conversion functions are borrowed from URDE https://github.com/AxioDL/urde/blob/master/DataSpec/DNACommon/TXTR.cpp
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

fn encode_rgb5a3(pixel: [u8; 4]) -> [u8; 2] {
    let v = if pixel[3] == 0xff {
        0x8000 | ((pixel[0] as u16 >> 3) << 10)
            | ((pixel[1] as u16 >> 3) << 5)
            | (pixel[2] as u16 >> 3)
    } else {
        ((pixel[0] as u16 >> 4) << 8)
            | ((pixel[1] as u16 >> 4) << 4)
            | (pixel[2] as u16 >> 4)
            | ((pixel[3] as u16 >> 5) << 12)
    };
    v.to_be_bytes()
}

fn encode_rgb565(pixel: [u8; 3]) -> [u8; 2] {
    let v = ((pixel[0] as u16 >> 3) << 11)
            | ((pixel[1] as u16 >> 2) << 5)
            | ((pixel[2] as u16 >> 3));
    v.to_be_bytes()
}

fn decode_rgb5a3(texel: [u8; 2]) -> [u8; 4] {
    let v = u16::from_be_bytes(texel);
    if v & 0x8000 != 0 {
        [
            convert5to8(((v >> 10) & 0x1f) as u8),
            convert5to8(((v >> 5) & 0x1f) as u8),
            convert5to8((v & 0x1f) as u8),
            0xff,
        ]
    } else {
        [
            convert4to8(((v >> 8) & 0xf) as u8),
            convert4to8(((v >> 4) & 0xf) as u8),
            convert4to8((v & 0xf) as u8),
            convert3to8((v >> 12 & 0x7) as u8),
        ]
    }
}

fn decode_rgb565(texel: [u8; 2]) -> [u8; 3] {
    let v = u16::from_be_bytes(texel);
    [
        convert5to8(((v >> 11) & 0x1f) as u8),
        convert6to8(((v >> 5) & 0x3f) as u8),
        convert5to8((v & 0x1f) as u8),
    ]
}

trait TxtrFormatExt {
    fn color_type(&self) -> ColorType;
    fn bytes_per_block(&self) -> usize;
    fn block_dimensions(&self) -> (usize, usize);
    fn flipped(&self) -> bool;
    fn compute_palette(&mut self, pixels: &[u8]) -> Result<(), ()>;
    fn decode_block(&self, block: &[u8], pixels: &mut[u8]);
    fn encode_block(&self, block: &mut [u8], pixels: &[u8]);
    fn from_str(s: &str) -> Result<TxtrFormat, ()>;
}

impl TxtrFormatExt for TxtrFormat {
    fn color_type(&self) -> ColorType {
        match self {
            TxtrFormat::I4 => ColorType::L8,
            TxtrFormat::I8 => ColorType::L8,
            TxtrFormat::Ia4 => ColorType::La8,
            TxtrFormat::Ia8 => ColorType::La8,
            TxtrFormat::C4(fmt, _) | TxtrFormat::C8(fmt, _) => match fmt {
                TxtrPaletteFormat::Ia8  => ColorType::La8,
                TxtrPaletteFormat::Rgb565 => ColorType::Rgb8,
                TxtrPaletteFormat::Rgb5A3 => ColorType::Rgba8,
            },
            TxtrFormat::Rgb565 => ColorType::Rgb8,
            TxtrFormat::Rgb5A3 => ColorType::Rgba8,
            TxtrFormat::Rgba8 => ColorType::Rgba8,
            TxtrFormat::Cmpr => ColorType::Rgba8,
        }
    }

    fn bytes_per_block(&self) -> usize {
        match self {
            TxtrFormat::I4 => 32,
            TxtrFormat::I8 => 32,
            TxtrFormat::Ia4 => 32,
            TxtrFormat::Ia8 => 32,
            TxtrFormat::C4(_, _) => 32,
            TxtrFormat::C8(_, _) => 32,
            TxtrFormat::Rgb565 => 32,
            TxtrFormat::Rgb5A3 => 32,
            TxtrFormat::Rgba8 => 64,
            TxtrFormat::Cmpr => 32,
        }
    }

    fn block_dimensions(&self) -> (usize, usize) {
        match self {
            TxtrFormat::I4 => (8, 8),
            TxtrFormat::I8 => (8, 4),
            TxtrFormat::Ia4 => (8, 4),
            TxtrFormat::Ia8 => (4, 4),
            TxtrFormat::C4(_, _) => (8, 8),
            TxtrFormat::C8(_, _) => (8, 4),
            TxtrFormat::Rgb565 => (4, 4),
            TxtrFormat::Rgb5A3 => (4, 4),
            TxtrFormat::Rgba8 => (4, 4),
            TxtrFormat::Cmpr => (8, 8),
        }
    }

    fn flipped(&self) -> bool {
        match self {
            TxtrFormat::C4(_, _) | TxtrFormat::C8(_, _) => false,
            _ => true,
        }
    }


    fn compute_palette(&mut self, pixels: &[u8]) -> Result<(), ()> {
        let (fmt, buf) = match self {
            TxtrFormat::C4(fmt, buf) => (fmt, &mut buf[..]),
            TxtrFormat::C8(fmt, buf) => (fmt, &mut buf[..]),
            _ => return Ok(()),
        };
        let mut palette_values = HashMap::with_capacity(buf.len() / 2);
        for chunk in pixels.chunks(fmt.color_type().channel_count() as usize) {
            let encoded = match fmt {
                TxtrPaletteFormat::Ia8 => chunk.try_into().unwrap(),
                TxtrPaletteFormat::Rgb565 => encode_rgb565(chunk.try_into().unwrap()),
                TxtrPaletteFormat::Rgb5A3 => encode_rgb5a3(chunk.try_into().unwrap()),
            };
            let pv_len = palette_values.len();
            let idx = *palette_values.entry(encoded)
                .or_insert(pv_len);
            if idx * 2 + 2 >= buf.len() {
                Err(())?;
            }
            buf[idx * 2..idx * 2 + 2].copy_from_slice(&encoded[..]);
        }
        Ok(())
    }

    fn decode_block(&self, block: &[u8], pixels: &mut[u8]) {
        assert_eq!(block.len(), self.bytes_per_block());
        let channel_count = self.color_type().channel_count() as usize;
        assert_eq!(
            pixels.len(),
            channel_count * self.block_dimensions().0 * self.block_dimensions().1,
        );
        match self {
            TxtrFormat::I4 => {
                for (texel_byte, pixel_bytes) in block.iter().zip(pixels.chunks_mut(2)) {
                    pixel_bytes[0] = convert4to8(texel_byte >> 4);
                    pixel_bytes[1] = convert4to8(texel_byte & 0xf);
                }
            },
            TxtrFormat::I8 => pixels.copy_from_slice(block),
            TxtrFormat::Ia4 => {
                for (texel_byte, pixel_bytes) in block.iter().zip(pixels.chunks_mut(2)) {
                    pixel_bytes[0] = convert4to8(texel_byte >> 4);
                    pixel_bytes[1] = convert4to8(texel_byte & 0xf);
                }
            },
            TxtrFormat::Ia8 => pixels.copy_from_slice(block),
            TxtrFormat::C4(fmt, palette) => {
                let iter = block.iter()
                    .flat_map(|texel| iter::once(texel >> 4).chain(iter::once(texel & 0xf)))
                    .map(|nibble| nibble as usize)
                    .zip(pixels.chunks_mut(self.color_type().channel_count() as usize));
                for (texel_nibble, pixel_bytes) in iter {
                    let texel_from_palette = &palette[texel_nibble * 2..texel_nibble * 2 + 2];
                    match fmt {
                        TxtrPaletteFormat::Ia8 => pixel_bytes.copy_from_slice(texel_from_palette),
                        TxtrPaletteFormat::Rgb565 => {
                            let decoded = decode_rgb565(texel_from_palette.try_into().unwrap());
                            pixel_bytes.copy_from_slice(&decoded[..]);
                        },
                        TxtrPaletteFormat::Rgb5A3 => {
                            let decoded = decode_rgb5a3(texel_from_palette.try_into().unwrap());
                            pixel_bytes.copy_from_slice(&decoded[..]);
                        },
                    }
                }
            },
            TxtrFormat::C8(fmt, palette) => {
                let iter = block.iter()
                    .map(|byte| *byte as usize)
                    .zip(pixels.chunks_mut(self.color_type().channel_count() as usize));
                for (texel_byte, pixel_bytes) in iter {
                    let texel_from_palette = &palette[texel_byte * 2..texel_byte * 2 + 2];
                    match fmt {
                        TxtrPaletteFormat::Ia8 => pixel_bytes.copy_from_slice(texel_from_palette),
                        TxtrPaletteFormat::Rgb565 => {
                            let decoded = decode_rgb565(texel_from_palette.try_into().unwrap());
                            pixel_bytes.copy_from_slice(&decoded[..]);
                        },
                        TxtrPaletteFormat::Rgb5A3 => {
                            let decoded = decode_rgb5a3(texel_from_palette.try_into().unwrap());
                            pixel_bytes.copy_from_slice(&decoded[..]);
                        },
                    }
                }
            },
            TxtrFormat::Rgb565 => {
                for (texel, pixel) in block.chunks(2).zip(pixels.chunks_mut(3)) {
                    pixel.copy_from_slice(&decode_rgb565(texel.try_into().unwrap())[..]);
                }
            },
            TxtrFormat::Rgb5A3 => {
                for (texel, pixel) in block.chunks(2).zip(pixels.chunks_mut(4)) {
                    pixel.copy_from_slice(&decode_rgb5a3(texel.try_into().unwrap())[..]);
                }
            },
            TxtrFormat::Rgba8 => pixels.copy_from_slice(block),
            TxtrFormat::Cmpr => {
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

    fn encode_block(&self, block: &mut [u8], pixels: &[u8]) {
        assert_eq!(block.len(), self.bytes_per_block());
        let channel_count = self.color_type().channel_count() as usize;
        assert_eq!(
            pixels.len(),
            channel_count * self.block_dimensions().0 * self.block_dimensions().1,
        );
        match self {
            TxtrFormat::I4 => {
                for (block_byte, pixel_bytes) in block.iter_mut().zip(pixels.chunks(2)) {
                    *block_byte = (pixel_bytes[1] >> 4) | ((pixel_bytes[0] >> 4) << 4);
                }
            },
            TxtrFormat::I8 => block.copy_from_slice(pixels),
            TxtrFormat::Ia4 => {
                for (block_byte, pixel_bytes) in block.iter_mut().zip(pixels.chunks(2)) {
                    *block_byte = (pixel_bytes[1] >> 4) | ((pixel_bytes[0] >> 4) << 4);
                }
            },
            TxtrFormat::Ia8 => block.copy_from_slice(pixels),
            TxtrFormat::C4(fmt, palette) => {
                let mut map = HashMap::with_capacity(palette.len() / 2);
                for (i, texel) in palette.chunks(2).enumerate() {
                    map.entry([texel[0], texel[1]])
                        .or_insert(i);
                }
                let cc = self.color_type().channel_count() as usize;
                for (texel, pixels) in block.iter_mut().zip(pixels.chunks(cc * 2)) {
                    let mut nibbles = [0; 2];
                    for (nibble, pixel) in nibbles.iter_mut().zip(pixels.chunks(cc)) {
                        let encoded = match fmt {
                            TxtrPaletteFormat::Ia8 => pixel.try_into().unwrap(),
                            TxtrPaletteFormat::Rgb565 => encode_rgb565(pixel.try_into().unwrap()),
                            TxtrPaletteFormat::Rgb5A3 => encode_rgb5a3(pixel.try_into().unwrap()),
                        };
                        *nibble = map[&encoded] as u8;
                    }
                    *texel = nibbles[1] | (nibbles[0] << 4);
                }
            },
            TxtrFormat::C8(fmt, palette) => {
                let mut map = HashMap::with_capacity(palette.len() / 2);
                for (i, texel) in palette.chunks(2).enumerate() {
                    map.insert([texel[0], texel[1]], i);
                }
                let cc = self.color_type().channel_count() as usize;
                for (texel, pixel) in block.iter_mut().zip(pixels.chunks(cc)) {
                    let encoded = match fmt {
                        TxtrPaletteFormat::Ia8 => pixel.try_into().unwrap(),
                        TxtrPaletteFormat::Rgb565 => encode_rgb565(pixel.try_into().unwrap()),
                        TxtrPaletteFormat::Rgb5A3 => encode_rgb5a3(pixel.try_into().unwrap()),
                    };
                    *texel = map[&encoded] as u8;
                }
            },
            TxtrFormat::Rgb565 => {
                for (texel, pixel) in block.chunks_mut(2).zip(pixels.chunks(3)) {
                    texel.copy_from_slice(&encode_rgb565(pixel.try_into().unwrap())[..]);
                }
            },
            TxtrFormat::Rgb5A3 => {
                for (texel, pixel) in block.chunks_mut(2).zip(pixels.chunks(4)) {
                    texel.copy_from_slice(&encode_rgb5a3(pixel.try_into().unwrap())[..]);
                }
            },
            TxtrFormat::Rgba8 => block.copy_from_slice(pixels),
            TxtrFormat::Cmpr => {
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

    fn from_str(s: &str) -> Result<TxtrFormat, ()> {
        match s.to_ascii_lowercase().as_str() {
            "i4" => Ok(TxtrFormat::I4),
            "i8" => Ok(TxtrFormat::I8),
            "ia4" => Ok(TxtrFormat::Ia4),
            "ia8" => Ok(TxtrFormat::Ia8),
            "c4(ia8)" => Ok(TxtrFormat::C4(TxtrPaletteFormat::Ia8, Default::default())),
            "c4(rgb565)" => Ok(TxtrFormat::C4(TxtrPaletteFormat::Rgb565, Default::default())),
            "c4(rgb5a3)" => Ok(TxtrFormat::C4(TxtrPaletteFormat::Rgb5A3, Default::default())),
            "c8(ia8)" => Ok(TxtrFormat::C8(TxtrPaletteFormat::Ia8, Default::default())),
            "c8(rgb565)" => Ok(TxtrFormat::C8(TxtrPaletteFormat::Rgb565, Default::default())),
            "c8(rgb5a3)" => Ok(TxtrFormat::C8(TxtrPaletteFormat::Rgb5A3, Default::default())),
            "rgb565" => Ok(TxtrFormat::Rgb565),
            "rgb5a3" => Ok(TxtrFormat::Rgb5A3),
            "rgba8" => Ok(TxtrFormat::Rgba8),
            "cmpr" => Ok(TxtrFormat::Cmpr),
            _ => Err(()),
        }
    }
}

trait TxtrPaletteFormatExt {
    fn color_type(&self) -> ColorType;
}

impl TxtrPaletteFormatExt for TxtrPaletteFormat {
    fn color_type(&self) -> ColorType {
        match self {
            TxtrPaletteFormat::Ia8 => ColorType::La8,
            TxtrPaletteFormat::Rgb565 => ColorType::Rgb8,
            TxtrPaletteFormat::Rgb5A3 => ColorType::Rgba8,
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
                { |s| s.parse::<u8>()
                    .map(|_| ())
                        .map_err(|_| "Expected integer for mipmap".into()) }
                "Which mipmap to extract. Defaults to 0 (full-size)."
            )
        )
        (@subcommand png2txtr =>
            (about: "Converts a PNG to a TXTR.")
            (@arg input: -i --input +takes_value +required "Input PNG file to convert.")
            (@arg output: -o --output +takes_value +required "Output path to write the TXTR file.")
            (@arg format: -f --format +takes_value +required
                {
                    |s| TxtrFormat::from_str(s.as_str())
                        .map(|_| ())
                        .map_err(|()| format!("Unknown format \"{}\"", s))
                }
                "TXTR format to use. Accepted values are: \
                 I4, I8, IA4, IA8, RGB565, RGB5A3, RGBA8, CMPR \
                 C4(IA8), C4(RGB565), C4(RGB5A3), \
                 C8(IA8), C8(RGB565), C8(RGB5A3), \
                 (case-insensitive)."
            )
            (@arg mipmap_count: -m --mipmap_count +takes_value
                { |s| s.parse::<u8>()
                    .map(|_| ())
                        .map_err(|_| "Expected integer for mipmap count".into()) }
                "Number of mipmaps to generate. Defaults to the maximum number for the \
                 image & format."
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
            matches.value_of("mipmap_count").map(|s| s.parse().unwrap()),
            TxtrFormat::from_str(matches.value_of("format").unwrap()).unwrap(),
        ),
        _ => return,
    };
    if let Err(s) = res {
        eprintln!("{} {}", clap::Format::Error("error:"), s);
    }
}
