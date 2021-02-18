use std::convert::TryInto;

use libsquish_wrapper::{compress_dxt1gcn_block, decompress_dxt1gcn_block};
use resource_info_table::{resource_info, ResourceInfo};


pub const VARIA_SUIT_TEXTURES: &[ResourceInfo] = &[
    resource_info!("gravity_head_chest.TXTR"),
    resource_info!("power_head_chest_incan.TXTR"),

    resource_info!("gravity_torso_ball.TXTR"),

    resource_info!("gravity_legs.TXTR"),
    resource_info!("gravity_legs_incan.TXTR"),

    resource_info!("gravity_arms.TXTR"),
    resource_info!("gravity_arms_incan.TXTR"),
];


struct CmprPixelIter {
    cnt: usize,
    width: usize,
    height: usize,
}

impl CmprPixelIter
{
    fn new(width: usize, height: usize) -> Self
    {
        CmprPixelIter {
            cnt: 0,
            width,
            height,
        }
    }
}

impl Iterator for CmprPixelIter
{
    type Item = (usize, usize);
    fn next(&mut self) -> Option<Self::Item>
    {
        let inner_x = self.cnt & 1;
        let inner_y = (self.cnt & 2) >> 1;
        let block_x = ((self.cnt & !3) >> 2) % (self.width / 8);
        let block_y = ((self.cnt & !3) >> 2) / (self.width / 8);

        let first_pixel_x = block_x * 8 + inner_x * 4;
        // TODO: Check for underflow, if we did, return None?
        let first_pixel_y = self.height - 4 - (block_y * 8 + inner_y * 4);
        self.cnt += 1;
        Some((first_pixel_x, first_pixel_y))
    }
}

pub fn cmpr_decompress(compressed: &[u8], width: usize, height: usize, decompressed: &mut [u8])
{
    let cmpr_iter = CmprPixelIter::new(width, height);
    for (chunk, (first_pixel_x, first_pixel_y)) in compressed.chunks_exact(8).zip(cmpr_iter) {
        let mut decompressed_pixels = [[0u8; 4]; 16];
        decompress_dxt1gcn_block(&mut decompressed_pixels, chunk.try_into().unwrap());
        for y in 0..4 {
            for x in 0..4 {
                let pixel_x = first_pixel_x + x;
                let pixel_y = first_pixel_y + (3 - y);
                let pixel_start = (pixel_y * width + pixel_x) * 4;
                let pixel_data = decompressed_pixels[y * 4 + x];
                decompressed[pixel_start..pixel_start + 4].copy_from_slice(&pixel_data);
            }
        }
    }
}

pub fn cmpr_compress(uncompressed: &[u8], width: usize, height: usize, compressed: &mut [u8])
{
    let cmpr_iter = CmprPixelIter::new(width, height);
    for (chunk, (first_pixel_x, first_pixel_y)) in compressed.chunks_exact_mut(8).zip(cmpr_iter) {

        let mut uncompressed_pixels = [[0u8; 4]; 16];

        for y in 0..4 {
            for x in 0..4 {
                let pixel_x = first_pixel_x + x;
                let pixel_y = first_pixel_y + (3 - y);
                let pixel_start = (pixel_y * width + pixel_x) * 4;
                uncompressed_pixels[y * 4 + x] = uncompressed[pixel_start..pixel_start + 4].try_into().unwrap();
            }
        }

        compress_dxt1gcn_block(&uncompressed_pixels, chunk.try_into().unwrap());
    }
}

// Adapted from image-rs
pub fn huerotate_in_place(image: &mut [u8], width: usize, height: usize, value: i32)
where
{
    let angle: f64 = value as f64;

    let cosv = (angle * std::f64::consts::PI / 180.0).cos();
    let sinv = (angle * std::f64::consts::PI / 180.0).sin();
    let matrix: [f64; 9] = [
        // Reds
        0.213 + cosv * 0.787 - sinv * 0.213,
        0.715 - cosv * 0.715 - sinv * 0.715,
        0.072 - cosv * 0.072 + sinv * 0.928,
        // Greens
        0.213 - cosv * 0.213 + sinv * 0.143,
        0.715 + cosv * 0.285 + sinv * 0.140,
        0.072 - cosv * 0.072 - sinv * 0.283,
        // Blues
        0.213 - cosv * 0.213 - sinv * 0.787,
        0.715 - cosv * 0.715 + sinv * 0.715,
        0.072 + cosv * 0.928 + sinv * 0.072,
    ];
    for y in 0..height {
        for x in 0..width {
            let start = (y * width + x) * 4;
            let pixel = &mut image[start..start + 4];
            let (k1, k2, k3, k4) = (pixel[0], pixel[1], pixel[2], pixel[3]);
            let vec: (f64, f64, f64, f64) = (k1 as f64, k2 as f64, k3 as f64, k4 as f64);

            let r = vec.0;
            let g = vec.1;
            let b = vec.2;

            let new_r = matrix[0] * r + matrix[1] * g + matrix[2] * b;
            let new_g = matrix[3] * r + matrix[4] * g + matrix[5] * b;
            let new_b = matrix[6] * r + matrix[7] * g + matrix[8] * b;
            let outpixel = [
                new_r.clamp(0.0, 255.0) as u8,
                new_g.clamp(0.0, 255.0) as u8,
                new_b.clamp(0.0, 255.0) as u8,
                vec.3.clamp(0.0, 255.0) as u8,
            ];

           pixel.copy_from_slice(&outpixel[..]);
        }
    }
}
