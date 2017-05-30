use std::mem;

use reader_writer::{FourCC, Reader};
use structs::{Connection, Pickup, Resource, ResourceKind};

pub struct PickupMeta
{
    pub pickup: Pickup<'static>,
    pub deps: &'static [(u32, FourCC)],
    pub hudmemo_strg: u32,
    pub attainment_audio_file_name: &'static str,
}

static mut _PICKUP_META: &'static [PickupMeta] = &[];

/// Leaks the memory held by a Vec and returns a static lifetime slice with that
/// data.
fn leak_vec<T>(vec: Vec<T>) -> &'static [T]
{
    let ptr = &*vec as *const [T];
    mem::forget(vec);
    unsafe { &*ptr }
}

/// This must be called before pickup_meta can be used.
pub fn setup_pickup_meta_table()
{
    let vec = PICKUP_RAW_META.iter()
        .map(|meta| {
            PickupMeta {
                pickup: Reader::new(meta.pickup).read(()),
                deps: leak_vec(meta.deps.iter().map(|&(fid, ref b)| (fid, b.into())).collect()),
                hudmemo_strg: meta.hudmemo_strg,
                attainment_audio_file_name: meta.attainment_audio_file_name,
            }
        })
        .collect();
    unsafe { _PICKUP_META = leak_vec(vec) };
}

pub fn pickup_meta_table()
    -> &'static [PickupMeta]
{
    debug_assert!(unsafe { _PICKUP_META }.len() == 36);
    unsafe { _PICKUP_META }
}

/// Lookup a pre-computed AABB for a pickup's CMDL
pub fn aabb_for_pickup_cmdl(cmdl_id: u32) -> Option<[f32; 6]>
{
    // The aabb array is sorted, so we can binary search.
    if let Ok(idx) = PICKUP_CMDL_AABBS.binary_search_by_key(&cmdl_id, |&(k, _)| k) {
        // The arrays contents are stored as u32s to reduce percision loss from
        // being converted to/from decimal literals. We use mem::transmute to
        // convert the u32s into f32s.
        Some(unsafe { mem::transmute(PICKUP_CMDL_AABBS[idx].1) })
    } else {
        None
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PickupLocation
{
    pub location: ScriptObjectLocation,
    pub attainment_audio: ScriptObjectLocation,
    pub hudmemo: Option<ScriptObjectLocation>,
    pub post_pickup_relay_connections: &'static [Connection]
}

#[derive(Clone, Copy, Debug)]
pub struct ScriptObjectLocation
{
    pub layer: u32,
    pub instance_id: u32,
}

const EXTRA_ASSETS: &'static [(u32, [u8; 4], &'static [u8])] = &[
    // Phazon Suit SCAN
    (0x50535343, *b"SCAN", include_bytes!("../extra_assets/phazon_suit_scan.scan")),
    // Phazon Suit STRG
    (0x50535353, *b"STRG", include_bytes!("../extra_assets/phazon_suit_scan.strg")),
    // Phazon Suit TXTR 1
    (0x50535431, *b"TXTR", include_bytes!("../extra_assets/phazon_suit_texure_1.txtr")),
    // Phazon Suit TXTR 2
    (0x50535432, *b"TXTR", include_bytes!("../extra_assets/phazon_suit_texure_2.txtr")),
    // Nothing acquired HudMemo STRG
    (0xDEAF0000, *b"STRG", include_bytes!("../extra_assets/nothing_hudmemo.strg")),
    // Nothing scan STRG
    (0xDEAF0001, *b"STRG", include_bytes!("../extra_assets/nothing_scan.strg")),
    // Nothing SCAN
    (0xDEAF0002, *b"SCAN", include_bytes!("../extra_assets/nothing_scan.scan")),
];

#[cfg(not(debug_assertions))]
pub fn build_resource<'a>(file_id: u32, kind: ResourceKind<'a>) -> Resource<'a>
{
    Resource {
        compressed: false,
        file_id: file_id,
        kind: kind,
    }
}

#[cfg(debug_assertions)]
pub fn build_resource<'a>(file_id: u32, kind: ResourceKind<'a>) -> Resource<'a>
{
    Resource {
        compressed: false,
        file_id: file_id,
        kind: kind,
        original_offset: 0,
    }
}
pub fn extra_assets<'a>() -> Vec<Resource<'a>>
{
    EXTRA_ASSETS.iter().map(|&(file_id, ref fourcc, bytes)| {
        build_resource(file_id, ResourceKind::Unknown(Reader::new(bytes), fourcc.into()))
    }).collect()
}

const MARKER_ASSERT_DATA: &'static [u8] = &[
    0x87, 0x65, 0x43, 0x21, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
    0x45, 0x4E, 0x47, 0x4C, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0xF0, 0x00, 0x00, 0x00, 0x04,
    0x00, 0x54, 0x00, 0x68, 0x00, 0x69, 0x00, 0x73,
    0x00, 0x20, 0x00, 0x69, 0x00, 0x73, 0x00, 0x20,
    0x00, 0x75, 0x00, 0x73, 0x00, 0x65, 0x00, 0x64,
    0x00, 0x20, 0x00, 0x62, 0x00, 0x79, 0x00, 0x20,
    0x00, 0x74, 0x00, 0x68, 0x00, 0x65, 0x00, 0x20,
    0x00, 0x52, 0x00, 0x61, 0x00, 0x6E, 0x00, 0x64,
    0x00, 0x6F, 0x00, 0x6D, 0x00, 0x69, 0x00, 0x7A,
    0x00, 0x65, 0x00, 0x72, 0x00, 0x20, 0x00, 0x74,
    0x00, 0x6F, 0x00, 0x20, 0x00, 0x64, 0x00, 0x65,
    0x00, 0x74, 0x00, 0x65, 0x00, 0x63, 0x00, 0x74,
    0x00, 0x20, 0x00, 0x66, 0x00, 0x69, 0x00, 0x6C,
    0x00, 0x65, 0x00, 0x73, 0x00, 0x20, 0x00, 0x61,
    0x00, 0x64, 0x00, 0x64, 0x00, 0x65, 0x00, 0x64,
    0x00, 0x20, 0x00, 0x74, 0x00, 0x6F, 0x00, 0x20,
    0x00, 0x74, 0x00, 0x68, 0x00, 0x65, 0x00, 0x20,
    0x00, 0x70, 0x00, 0x61, 0x00, 0x6B, 0x00, 0x2C,
    0x00, 0x20, 0x00, 0x73, 0x00, 0x6F, 0x00, 0x20,
    0x00, 0x74, 0x00, 0x68, 0x00, 0x65, 0x00, 0x79,
    0x00, 0x20, 0x00, 0x63, 0x00, 0x61, 0x00, 0x6E,
    0x00, 0x20, 0x00, 0x62, 0x00, 0x65, 0x00, 0x20,
    0x00, 0x72, 0x00, 0x65, 0x00, 0x6D, 0x00, 0x6F,
    0x00, 0x76, 0x00, 0x65, 0x00, 0x64, 0x00, 0x20,
    0x00, 0x69, 0x00, 0x6E, 0x00, 0x20, 0x00, 0x73,
    0x00, 0x75, 0x00, 0x62, 0x00, 0x73, 0x00, 0x65,
    0x00, 0x71, 0x00, 0x75, 0x00, 0x65, 0x00, 0x6E,
    0x00, 0x74, 0x00, 0x20, 0x00, 0x72, 0x00, 0x61,
    0x00, 0x6E, 0x00, 0x64, 0x00, 0x6F, 0x00, 0x6D,
    0x00, 0x69, 0x00, 0x7A, 0x00, 0x61, 0x00, 0x74,
    0x00, 0x69, 0x00, 0x6F, 0x00, 0x6E, 0x00, 0x73,
    0x00, 0x2E, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
];

#[cfg(not(debug_assertions))]
pub fn marker_asset<'a>() -> Resource<'a>
{
    Resource {
        compressed: false,
        file_id: 0x53465A4E,
        kind: ResourceKind::Unknown(Reader::new(MARKER_ASSERT_DATA), b"STRG".into()),
    }
}

#[cfg(debug_assertions)]
pub fn marker_asset<'a>() -> Resource<'a>
{
    Resource {
        compressed: false,
        file_id: 0x53465A4E,
        kind: ResourceKind::Unknown(Reader::new(MARKER_ASSERT_DATA), b"STRG".into()),
        original_offset: 0,
    }
}

struct PickupMetaRaw
{
    pickup: &'static [u8],
    deps: &'static [(u32, [u8; 4])],
    hudmemo_strg: u32,
    attainment_audio_file_name: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub struct RoomInfo
{
    pub room_id: u32,
    pub pickup_locations: &'static [PickupLocation],
    pub objects_to_remove: &'static [ObjectsToRemove],
}

#[derive(Clone, Copy, Debug)]
pub struct ObjectsToRemove
{
    pub layer: u32,
    pub instance_ids: &'static [u32],
}

include!("pickup_meta.rs.in");
