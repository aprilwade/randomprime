pub const REL_LOADER_100: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/rel_loader_1.00.bin"));
pub const REL_LOADER_101: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/rel_loader_1.01.bin"));
pub const REL_LOADER_102: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/rel_loader_1.02.bin"));
pub const REL_LOADER_PAL: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/rel_loader_pal.bin"));
pub const REL_LOADER_KOR: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/rel_loader_kor.bin"));
pub const REL_LOADER_JAP: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/rel_loader_jap.bin"));
pub const REL_LOADER_100_MAP: &str = include_str!(concat!(
        env!("OUT_DIR"),
        "/rel_loader_1.00.bin.map"
    ));
pub const REL_LOADER_101_MAP: &str = include_str!(concat!(
        env!("OUT_DIR"),
        "/rel_loader_1.01.bin.map"
    ));
pub const REL_LOADER_102_MAP: &str = include_str!(concat!(
        env!("OUT_DIR"),
        "/rel_loader_1.02.bin.map"
    ));
pub const REL_LOADER_PAL_MAP: &str = include_str!(concat!(
        env!("OUT_DIR"),
        "/rel_loader_pal.bin.map"
    ));
pub const REL_LOADER_KOR_MAP: &str = include_str!(concat!(
        env!("OUT_DIR"),
        "/rel_loader_kor.bin.map"
    ));
pub const REL_LOADER_JAP_MAP: &str = include_str!(concat!(
        env!("OUT_DIR"),
        "/rel_loader_jap.bin.map"
    ));
pub const PATCHES_100_REL: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/patches_1.00.rel"));
pub const PATCHES_101_REL: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/patches_1.01.rel"));
pub const PATCHES_102_REL: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/patches_1.02.rel"));
pub const PATCHES_PAL_REL: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/patches_pal.rel"));
pub const PATCHES_KOR_REL: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/patches_kor.rel"));
pub const PATCHES_JAP_REL: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/patches_jap.rel"));
