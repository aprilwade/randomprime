
extern "C" {

    // fn libsquish_CompressMasked(rgba: *const u8, mask: i32, block: *const u8, flags: i32, metric: *mut f32);
    fn libsquish_Compress(rgba: *const u8, block: *mut u8, flags: i32, metric: *mut f32);
    fn libsquish_Decompress(rgba: *mut u8, block: *const u8, flags: i32);
    // fn libsquish_GetStorageRequirements(width: i32, height: i32, flags: i32) -> i32;
    // fn libsquish_CompressImage(rgba: *const u8, width: i32, height: i32, blocks: *mut u8, flags: i32, metric: *mut f32);

    // fn libsquish_DecompressImage(rgba: *mut u8, width: i32, height: i32, blocks: *const u8, flags: i32);
}

pub fn compress_dxt1gcn_block(rgba: &[[u8; 4]; 16], block: &mut [u8; 8])
{
    unsafe {
        libsquish_Compress(
            rgba.as_ptr().cast(),
            block.as_mut_ptr(),
            ( 1 << 9 )/* kDxt1GCN */ | ( 1 << 8 ) /* kColourIterativeClusterFit */,
            std::ptr::null_mut(),
        )
    }
}

pub fn decompress_dxt1gcn_block(rgba: &mut [[u8; 4]; 16], block: &[u8; 8])
{
    assert!(block.len() >= 8);
    unsafe {
        libsquish_Decompress(
            rgba.as_mut_ptr().cast(),
            block.as_ptr(),
            1 << 9 /* kDxt1GCN */,
        )
    }
}
