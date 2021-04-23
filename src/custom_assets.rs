use resource_info_table::resource_info;
use reader_writer::{
    FourCC,
    Reader,
    Writable,
};
use structs::{res_id, ResId, Resource, ResourceKind};

use crate::{
    pickup_meta::{self, PickupType},
    door_meta::{DoorType, BlastShieldType},
    ResourceData,
    GcDiscLookupExtensions,
};

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

macro_rules! def_asset_ids {
    (@Build { $prev:expr } $id:ident: $fc:ident, $($rest:tt)*) => {
        def_asset_ids!(@Build { $prev } $id: $fc = $prev + 1, $($rest)*);
    };
    (@Build { $_prev:expr } $id:ident: $fc:ident = $e:expr, $($rest:tt)*) => {
        pub const $id: structs::ResId<structs::res_id::$fc> = structs::ResId::new($e);
        def_asset_ids!(@Build { $id.to_u32() } $($rest)*);
    };
    (@Build { $prev:expr }) => {
    };
    ($($tokens:tt)*) => {
        def_asset_ids!(@Build { 0 } $($tokens)*);
    };
}

pub mod custom_asset_ids {
    def_asset_ids! {
        // Item Assets //
        PHAZON_SUIT_SCAN: SCAN = 0xDEAF0000,
        PHAZON_SUIT_STRG: STRG,
        PHAZON_SUIT_TXTR1: TXTR,
        PHAZON_SUIT_TXTR2: TXTR,
        PHAZON_SUIT_CMDL: CMDL,
        PHAZON_SUIT_ANCS: ANCS,
        NOTHING_ACQUIRED_HUDMEMO_STRG: STRG,
        NOTHING_SCAN_STRG: STRG, // 0xDEAF0007
        NOTHING_SCAN: SCAN,
        NOTHING_TXTR: TXTR,
        NOTHING_CMDL: CMDL,
        NOTHING_ANCS: ANCS,
        THERMAL_VISOR_SCAN: SCAN,
        THERMAL_VISOR_STRG: STRG,
        SCAN_VISOR_ACQUIRED_HUDMEMO_STRG: STRG,
        SCAN_VISOR_SCAN_STRG: STRG,
        SCAN_VISOR_SCAN: SCAN,
        SHINY_MISSILE_TXTR0: TXTR,
        SHINY_MISSILE_TXTR1: TXTR,
        SHINY_MISSILE_TXTR2: TXTR,
        SHINY_MISSILE_CMDL: CMDL,
        SHINY_MISSILE_ANCS: ANCS,
        SHINY_MISSILE_EVNT: EVNT,
        SHINY_MISSILE_ANIM: ANIM,
        SHINY_MISSILE_ACQUIRED_HUDMEMO_STRG: STRG,
        SHINY_MISSILE_SCAN_STRG: STRG,
        SHINY_MISSILE_SCAN: SCAN,
        STARTING_ITEMS_HUDMEMO_STRG: STRG,

        // Door Assets //
        MORPH_BALL_BOMB_DOOR_CMDL: CMDL,
        POWER_BOMB_DOOR_CMDL: CMDL,
        MISSILE_DOOR_CMDL: CMDL,
        CHARGE_DOOR_CMDL: CMDL,
        SUPER_MISSILE_DOOR_CMDL: CMDL,
        WAVEBUSTER_DOOR_CMDL: CMDL,
        ICESPREADER_DOOR_CMDL: CMDL,
        FLAMETHROWER_DOOR_CMDL: CMDL,
        DISABLED_DOOR_CMDL: CMDL,
        AI_DOOR_CMDL: CMDL,

        VERTICAL_RED_DOOR_CMDL: CMDL,
        VERTICAL_POWER_BOMB_DOOR_CMDL: CMDL,
        VERTICAL_MORPH_BALL_BOMB_DOOR_CMDL: CMDL,
        VERTICAL_MISSILE_DOOR_CMDL: CMDL,
        VERTICAL_CHARGE_DOOR_CMDL: CMDL,
        VERTICAL_SUPER_MISSILE_DOOR_CMDL: CMDL,
        VERTICAL_DISABLED_DOOR_CMDL: CMDL,
        VERTICAL_WAVEBUSTER_DOOR_CMDL: CMDL,
        VERTICAL_ICESPREADER_DOOR_CMDL: CMDL,
        VERTICAL_FLAMETHROWER_DOOR_CMDL: CMDL,
        VERTICAL_AI_DOOR_CMDL: CMDL,

        MORPH_BALL_BOMB_DOOR_TXTR: TXTR,
        POWER_BOMB_DOOR_TXTR: TXTR,
        MISSILE_DOOR_TXTR: TXTR,
        CHARGE_DOOR_TXTR: TXTR,
        SUPER_MISSILE_DOOR_TXTR: TXTR,
        WAVEBUSTER_DOOR_TXTR: TXTR,
        ICESPREADER_DOOR_TXTR: TXTR,
        FLAMETHROWER_DOOR_TXTR: TXTR,
        DISABLED_DOOR_TXTR: TXTR,
        AI_DOOR_TXTR: TXTR,

        // blast shield assets //
        POWER_BOMB_BLAST_SHIELD_CMDL: CMDL,
        SUPER_BLAST_SHIELD_CMDL: CMDL,
        WAVEBUSTER_BLAST_SHIELD_CMDL: CMDL,
        ICESPREADER_BLAST_SHIELD_CMDL: CMDL,
        FLAMETHROWER_BLAST_SHIELD_CMDL: CMDL,

        BLAST_SHIELD_ALT_TXTR0: TXTR,
        BLAST_SHIELD_ALT_TXTR1: TXTR,
        BLAST_SHIELD_ALT_TXTR2: TXTR,

        POWER_BOMB_BLAST_SHIELD_TXTR: TXTR,
        SUPER_BLAST_SHIELD_TXTR: TXTR,
        WAVEBUSTER_BLAST_SHIELD_TXTR: TXTR,
        ICESPREADER_BLAST_SHIELD_TXTR: TXTR,
        FLAMETHROWER_BLAST_SHIELD_TXTR: TXTR,

        POWER_BOMB_BLAST_SHIELD_SCAN: SCAN,
        SUPER_BLAST_SHIELD_SCAN: SCAN,
        WAVEBUSTER_BLAST_SHIELD_SCAN: SCAN,
        ICESPREADER_BLAST_SHIELD_SCAN: SCAN,
        FLAMETHROWER_BLAST_SHIELD_SCAN: SCAN,

        POWER_BOMB_BLAST_SHIELD_STRG: STRG,
        SUPER_BLAST_SHIELD_STRG: STRG,
        WAVEBUSTER_BLAST_SHIELD_STRG: STRG,
        ICESPREADER_BLAST_SHIELD_STRG: STRG,
        FLAMETHROWER_BLAST_SHIELD_STRG: STRG,

        // has to be at the end //
        SKIP_HUDMEMO_STRG_START: STRG,
        SKIP_HUDMEMO_STRG_END: STRG = SKIP_HUDMEMO_STRG_START.to_u32() + 38,
        
        END: STRG = SKIP_HUDMEMO_STRG_END.to_u32(),
    }
}

pub fn build_resource<'r, K>(file_id: ResId<K>, kind: ResourceKind<'r>) -> Resource<'r>
    where K: res_id::ResIdKind,
{
    assert_eq!(K::FOURCC, kind.fourcc());
    build_resource_raw(file_id.to_u32(), kind)
}

#[cfg(not(debug_assertions))]
pub fn build_resource_raw<'r>(file_id: u32, kind: ResourceKind<'r>) -> Resource<'r>
{
    Resource {
        compressed: false,
        file_id,
        kind,
    }
}

#[cfg(debug_assertions)]
pub fn build_resource_raw<'r>(file_id: u32, kind: ResourceKind<'r>) -> Resource<'r>
{
    Resource {
        compressed: false,
        file_id,
        kind,
        original_offset: 0,
    }
}

// Assets defined in an external file
fn extern_assets<'r>() -> Vec<Resource<'r>>
{
    let extern_assets: &[(ResId<res_id::TXTR>, [u8; 4], &[u8])] = &[
        (custom_asset_ids::PHAZON_SUIT_TXTR1,         *b"TXTR", include_bytes!("../extra_assets/phazon_suit_texure_1.txtr")),
        (custom_asset_ids::PHAZON_SUIT_TXTR2,         *b"TXTR", include_bytes!("../extra_assets/phazon_suit_texure_2.txtr")),
        (custom_asset_ids::NOTHING_TXTR,              *b"TXTR", include_bytes!("../extra_assets/nothing_texture.txtr")),
        (custom_asset_ids::SHINY_MISSILE_TXTR0,       *b"TXTR", include_bytes!("../extra_assets/shiny-missile0.txtr")),
        (custom_asset_ids::SHINY_MISSILE_TXTR1,       *b"TXTR", include_bytes!("../extra_assets/shiny-missile1.txtr")),
        (custom_asset_ids::SHINY_MISSILE_TXTR2,       *b"TXTR", include_bytes!("../extra_assets/shiny-missile2.txtr")),
        (custom_asset_ids::AI_DOOR_TXTR,              *b"TXTR", include_bytes!("../extra_assets/holorim_ai.txtr")),
        (custom_asset_ids::MORPH_BALL_BOMB_DOOR_TXTR, *b"TXTR", include_bytes!("../extra_assets/holorim_bombs.txtr")),
        (custom_asset_ids::POWER_BOMB_DOOR_TXTR,      *b"TXTR", include_bytes!("../extra_assets/holorim_powerbomb.txtr")),
        (custom_asset_ids::SUPER_MISSILE_DOOR_TXTR,   *b"TXTR", include_bytes!("../extra_assets/holorim_super.txtr")),
        (custom_asset_ids::WAVEBUSTER_DOOR_TXTR,      *b"TXTR", include_bytes!("../extra_assets/holorim_wavebuster.txtr")),
        (custom_asset_ids::ICESPREADER_DOOR_TXTR,     *b"TXTR", include_bytes!("../extra_assets/holorim_icespreader.txtr")),
        (custom_asset_ids::FLAMETHROWER_DOOR_TXTR,    *b"TXTR", include_bytes!("../extra_assets/holorim_flamethrower.txtr")),
        (custom_asset_ids::BLAST_SHIELD_ALT_TXTR0,         *b"TXTR", include_bytes!("../extra_assets/blast_shield_alt_0.txtr")),
        (custom_asset_ids::BLAST_SHIELD_ALT_TXTR1,         *b"TXTR", include_bytes!("../extra_assets/blast_shield_alt_1.txtr")),
        (custom_asset_ids::BLAST_SHIELD_ALT_TXTR2,         *b"TXTR", include_bytes!("../extra_assets/blast_shield_alt_2.txtr")),
        (custom_asset_ids::POWER_BOMB_BLAST_SHIELD_TXTR,   *b"TXTR", include_bytes!("../extra_assets/blast_shield_pbm.txtr")),
        (custom_asset_ids::SUPER_BLAST_SHIELD_TXTR,        *b"TXTR", include_bytes!("../extra_assets/blast_shield_spr.txtr")),
        (custom_asset_ids::WAVEBUSTER_BLAST_SHIELD_TXTR,   *b"TXTR", include_bytes!("../extra_assets/blast_shield_wvb.txtr")),
        (custom_asset_ids::ICESPREADER_BLAST_SHIELD_TXTR,  *b"TXTR", include_bytes!("../extra_assets/blast_shield_ice.txtr")),
        (custom_asset_ids::FLAMETHROWER_BLAST_SHIELD_TXTR, *b"TXTR", include_bytes!("../extra_assets/blast_shield_flm.txtr")),
    ];

    extern_assets.iter().map(|&(res, ref fourcc, bytes)| {
        build_resource(res, ResourceKind::Unknown(Reader::new(bytes), fourcc.into()))
    }).collect()
}

// Assets not found in the base game
pub fn custom_assets<'r>(
    resources: &HashMap<(u32, FourCC),
    structs::Resource<'r>>,
    starting_memo: Option<&str>,
) -> Vec<Resource<'r>>
{
    // External assets
    let mut assets = extern_assets();

    // Custom pickup model assets
    assets.extend_from_slice(&create_suit_icon_cmdl_and_ancs(
        resources,
        custom_asset_ids::NOTHING_CMDL,
        custom_asset_ids::NOTHING_ANCS,
        custom_asset_ids::NOTHING_TXTR,
        custom_asset_ids::PHAZON_SUIT_TXTR2,
    ));
    assets.extend_from_slice(&create_suit_icon_cmdl_and_ancs(
        resources,
        custom_asset_ids::PHAZON_SUIT_CMDL,
        custom_asset_ids::PHAZON_SUIT_ANCS,
        custom_asset_ids::PHAZON_SUIT_TXTR1,
        custom_asset_ids::PHAZON_SUIT_TXTR2,
    ));
    assets.extend_from_slice(&create_item_scan_strg_pair(
        custom_asset_ids::PHAZON_SUIT_SCAN,
        custom_asset_ids::PHAZON_SUIT_STRG,
        vec!["Phazon Suit\0".to_string()],
    ));
    assets.extend_from_slice(&create_item_scan_strg_pair(
        custom_asset_ids::NOTHING_SCAN,
        custom_asset_ids::NOTHING_SCAN_STRG,
        vec!["???\0".to_string()],
    ));
    assets.push(build_resource(
        custom_asset_ids::NOTHING_ACQUIRED_HUDMEMO_STRG,
        structs::ResourceKind::Strg(structs::Strg::from_strings(vec![
            "&just=center;Nothing acquired!\0".to_owned(),
        ])),
    ));
    assets.extend_from_slice(&create_item_scan_strg_pair(
        custom_asset_ids::THERMAL_VISOR_SCAN,
        custom_asset_ids::THERMAL_VISOR_STRG,
        vec!["Thermal Visor\0".to_string()],
    ));
    assets.extend_from_slice(&create_item_scan_strg_pair(
        custom_asset_ids::SCAN_VISOR_SCAN,
        custom_asset_ids::SCAN_VISOR_SCAN_STRG,
        vec!["Scan Visor\0".to_string()],
    ));
    assets.push(build_resource(
        custom_asset_ids::SCAN_VISOR_ACQUIRED_HUDMEMO_STRG,
        structs::ResourceKind::Strg(structs::Strg::from_strings(vec![
            "&just=center;Scan Visor acquired!\0".to_owned(),
        ])),
    ));
    assets.extend_from_slice(&create_item_scan_strg_pair(
        custom_asset_ids::SHINY_MISSILE_SCAN,
        custom_asset_ids::SHINY_MISSILE_SCAN_STRG,
        vec!["Shiny Missile\0".to_string()],
    ));
    assets.extend_from_slice(&create_shiny_missile_assets(resources));
    assets.push(build_resource(
        custom_asset_ids::SHINY_MISSILE_ACQUIRED_HUDMEMO_STRG,
        structs::ResourceKind::Strg(structs::Strg::from_strings(vec![
            "&just=center;Shiny Missile acquired!\0".to_owned(),
        ])),
    ));

    if starting_memo.is_some() {
        assets.push(build_resource(
            custom_asset_ids::STARTING_ITEMS_HUDMEMO_STRG,
            structs::ResourceKind::Strg(structs::Strg::from_strings(vec![
                format!("&just=center;{}\0", starting_memo.clone().unwrap()),
            ])),
        ));
    }

    for pt in PickupType::iter() {
        let id = pt.skip_hudmemos_strg();
        assets.push(build_resource(
            id,
            structs::ResourceKind::Strg(structs::Strg {
                string_tables: vec![
                    structs::StrgStringTable {
                        lang: b"ENGL".into(),
                        strings: vec![format!("&just=center;{} acquired!\u{0}",
                                              pt.name()).into()].into(),
                    },
                ].into(),
            })
        ));
    }

    // Custom door assets
    for door_type in DoorType::iter() {
        if door_type.shield_cmdl().to_u32() >= 0xDEAF0000 && door_type.shield_cmdl().to_u32() <= custom_asset_ids::END.to_u32() { // only if it doesn't exist in-game already
            assets.push(create_custom_door_cmdl(resources, door_type));
        }
    }

    // Custom blast shield assets
    for blast_shield in BlastShieldType::iter() {
        if blast_shield.cmdl().to_u32() >= 0xDEAF0000 && blast_shield.cmdl().to_u32() <= custom_asset_ids::END.to_u32() { // only if it doesn't exist in-game already
            assets.push(create_custom_blast_shield_cmdl(resources, blast_shield));

            if blast_shield.scan() != ResId::invalid() && blast_shield.strg() != ResId::invalid() {
                assets.extend_from_slice(&create_item_scan_strg_pair(
                    blast_shield.scan(),
                    blast_shield.strg(),
                    blast_shield.scan_text(),
                ));
            }
        } else {
            // If vanilla CMDL, then it can't depend on custom textures 
            assert!(
                blast_shield.dependencies()
                .iter()
                .find(|d| d.0 >= 0xDEAF0000 && d.0 <= custom_asset_ids::END.to_u32())
                .is_none()
            );
        }
    }

    assets
}

// When modifying resources in an MREA, we need to give the room a copy of the resources/
// assests used b. Create a cache of all the resources needed by any pickup, door, etc...
pub fn collect_game_resources<'r>(
    gc_disc: &structs::GcDisc<'r>,
    starting_memo: Option<&str>,
)
    -> HashMap<(u32, FourCC), structs::Resource<'r>>
{
    // Get list of all dependencies patcher needs //
    let mut looking_for = HashSet::<_>::new();
    looking_for.extend(PickupType::iter().flat_map(|x| x.dependencies().iter().cloned()));
    looking_for.extend(PickupType::iter().map(|x| -> (_, _) { x.hudmemo_strg().into() }));
    looking_for.extend(DoorType::iter().flat_map(|x| x.dependencies()));
    looking_for.extend(BlastShieldType::iter().flat_map(|x| x.dependencies()));

    // Dependencies read from paks and custom assets will go here //
    let mut found = HashMap::with_capacity(looking_for.len());

    // Iterate through every level Pak //
    for pak_name in pickup_meta::ROOM_INFO.iter().map(|(name, _)| name) {
        let file_entry = gc_disc.find_file(pak_name).unwrap();
        let pak = match *file_entry.file().unwrap() {
            structs::FstEntryFile::Pak(ref pak) => Cow::Borrowed(pak),
            structs::FstEntryFile::Unknown(ref reader) => Cow::Owned(reader.clone().read(())),
            _ => panic!(),
        };

        // Iterate through all resources in level Pak //
        for res in pak.resources.iter() {
            // If this resource is a dependency needed by the patcher, add the resource to the output list //
            let key = (res.file_id, res.fourcc());
            if looking_for.remove(&key) {
                found.insert(key, res.into_owned());
            }
        }
    }

    // Remove extra assets from dependency search since they won't appear     //
    // in any pak. Instead add them to the output resource pool. These assets //
    // are provided as external files checked into the repository.            //
    for res in custom_assets(&found, starting_memo) {
        let key = (res.file_id, res.fourcc());
        looking_for.remove(&key);
        found.insert(key, res);
    }

    if !looking_for.is_empty() {
        panic!("error - still looking for {:?}", looking_for);
    }

    found
}

fn create_custom_blast_shield_cmdl<'r>(
    resources: &HashMap<(u32, FourCC),
    structs::Resource<'r>>,
    blast_shield_type: BlastShieldType,
) -> structs::Resource<'r>
{
    // Find and read the vanilla blast shield cmdl
    let old_cmdl = ResourceData::new(&resources[&resource_info!("EFDFFB8C.CMDL").into()]);

    // Create a copy 
    let old_cmdl_bytes = old_cmdl.decompress().into_owned();
    let mut new_cmdl = Reader::new(&old_cmdl_bytes[..]).read::<structs::Cmdl>(());

    // Modify the new CMDL to use custom textures
    new_cmdl.material_sets.as_mut_vec()[0].texture_ids.as_mut_vec()[0] = blast_shield_type.glow_border_txtr();
    new_cmdl.material_sets.as_mut_vec()[0].texture_ids.as_mut_vec()[1] = blast_shield_type.glow_trim_txtr();
    new_cmdl.material_sets.as_mut_vec()[0].texture_ids.as_mut_vec()[2] = blast_shield_type.metal_body_txtr();
    new_cmdl.material_sets.as_mut_vec()[0].texture_ids.as_mut_vec()[3] = blast_shield_type.animated_glow_txtr();
    new_cmdl.material_sets.as_mut_vec()[0].texture_ids.as_mut_vec()[4] = blast_shield_type.metal_trim_txtr();

    // Re-serialize the CMDL
    let mut new_cmdl_bytes = vec![];
    new_cmdl.write_to(&mut new_cmdl_bytes).unwrap();

    // Pad length to multiple of 32 bytes
    new_cmdl_bytes.extend(reader_writer::pad_bytes(32, new_cmdl_bytes.len()).iter());

    // Return resource
    build_resource(
        blast_shield_type.cmdl(),
        structs::ResourceKind::External(new_cmdl_bytes, b"CMDL".into())
    )
}

fn create_custom_door_cmdl<'r>(
    resources: &HashMap<(u32, FourCC),
    structs::Resource<'r>>,
    door_type: DoorType,
) -> structs::Resource<'r>
{
    let new_cmdl_id: ResId<res_id::CMDL> = door_type.shield_cmdl();
    let new_txtr_id: ResId<res_id::TXTR> = door_type.holorim_texture();

    let new_door_cmdl = {
        // Find and read the blue door CMDL
        let blue_door_cmdl = {
            if door_type.is_vertical() {
                ResourceData::new(&resources[&resource_info!("18D0AEE6.CMDL").into()]) // actually white door but who cares
            } else {
                ResourceData::new(&resources[&resource_info!("blueShield_v1.CMDL").into()])
            }
        };

        // Deserialize the blue door CMDL into a new mutable CMDL
        let blue_door_cmdl_bytes = blue_door_cmdl.decompress().into_owned();
        let mut new_cmdl = Reader::new(&blue_door_cmdl_bytes[..]).read::<structs::Cmdl>(());

        // Modify the new CMDL to make it unique
        new_cmdl.material_sets.as_mut_vec()[0].texture_ids.as_mut_vec()[0] = new_txtr_id;

        // Re-serialize the CMDL //
        let mut new_cmdl_bytes = vec![];
        new_cmdl.write_to(&mut new_cmdl_bytes).unwrap();

        // Pad length to multiple of 32 bytes //
        let len = new_cmdl_bytes.len();
        new_cmdl_bytes.extend(reader_writer::pad_bytes(32, len).iter());

        // Assemble into a proper resource object
        crate::custom_assets::build_resource(
            new_cmdl_id, // Custom ids start with 0xDEAFxxxx
            structs::ResourceKind::External(new_cmdl_bytes, b"CMDL".into())
        )
    };

    new_door_cmdl
}

fn create_suit_icon_cmdl_and_ancs<'r>(
    resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
    new_cmdl_id: ResId<res_id::CMDL>,
    new_ancs_id: ResId<res_id::ANCS>,
    new_txtr1: ResId<res_id::TXTR>,
    new_txtr2: ResId<res_id::TXTR>,
) -> [structs::Resource<'r>; 2]
{
    let new_suit_cmdl = {
        let grav_suit_cmdl = ResourceData::new(
            &resources[&resource_info!("Node1_11.CMDL").into()]
        );
        let cmdl_bytes = grav_suit_cmdl.decompress().into_owned();
        let mut cmdl = Reader::new(&cmdl_bytes[..]).read::<structs::Cmdl>(());

        cmdl.material_sets.as_mut_vec()[0].texture_ids.as_mut_vec()[0] = new_txtr1;
        cmdl.material_sets.as_mut_vec()[0].texture_ids.as_mut_vec()[3] = new_txtr2;

        let mut new_cmdl_bytes = vec![];
        cmdl.write_to(&mut new_cmdl_bytes).unwrap();

        build_resource(
            new_cmdl_id,
            structs::ResourceKind::External(new_cmdl_bytes, b"CMDL".into())
        )
    };
    let new_suit_ancs = {
        let grav_suit_ancs = ResourceData::new(
            &resources[&resource_info!("Node1_11.ANCS").into()]
        );
        let ancs_bytes = grav_suit_ancs.decompress().into_owned();
        let mut ancs = Reader::new(&ancs_bytes[..]).read::<structs::Ancs>(());

        ancs.char_set.char_info.as_mut_vec()[0].cmdl = new_cmdl_id;

        let mut new_ancs_bytes = vec![];
        ancs.write_to(&mut new_ancs_bytes).unwrap();

        build_resource(
            new_ancs_id,
            structs::ResourceKind::External(new_ancs_bytes, b"ANCS".into())
        )
    };
    [new_suit_cmdl, new_suit_ancs]
}

fn create_shiny_missile_assets<'r>(
    resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
) -> [structs::Resource<'r>; 4]
{
    let shiny_missile_cmdl = {
        let shiny_missile_cmdl = ResourceData::new(
            &resources[&resource_info!("Node1_36_0.CMDL").into()]
        );
        let cmdl_bytes = shiny_missile_cmdl.decompress().into_owned();
        let mut cmdl = Reader::new(&cmdl_bytes[..]).read::<structs::Cmdl>(());

        // println!("{:#?}", cmdl);
        cmdl.material_sets.as_mut_vec()[0].texture_ids = vec![
            custom_asset_ids::SHINY_MISSILE_TXTR0,
            custom_asset_ids::SHINY_MISSILE_TXTR1,
            custom_asset_ids::SHINY_MISSILE_TXTR2,
        ].into();

        let mut new_cmdl_bytes = vec![];
        cmdl.write_to(&mut new_cmdl_bytes).unwrap();

        build_resource(
            custom_asset_ids::SHINY_MISSILE_CMDL,
            structs::ResourceKind::External(new_cmdl_bytes, b"CMDL".into())
        )
    };
    let shiny_missile_ancs = {
        let shiny_missile_ancs = ResourceData::new(
            &resources[&resource_info!("Node1_37_0.ANCS").into()]
        );
        let ancs_bytes = shiny_missile_ancs.decompress().into_owned();
        let mut ancs = Reader::new(&ancs_bytes[..]).read::<structs::Ancs>(());

        ancs.char_set.char_info.as_mut_vec()[0].cmdl = custom_asset_ids::SHINY_MISSILE_CMDL;
        ancs.char_set.char_info.as_mut_vec()[0].particles.part_assets = vec![
            resource_info!("healthnew.PART").res_id
        ].into();
        if let Some(animation_resources) = &mut ancs.anim_set.animation_resources {
            animation_resources.as_mut_vec()[0].evnt = custom_asset_ids::SHINY_MISSILE_EVNT;
            animation_resources.as_mut_vec()[0].anim = custom_asset_ids::SHINY_MISSILE_ANIM;
        }

        match &mut ancs.anim_set.animations.as_mut_vec()[..] {
            [structs::Animation { meta: structs::MetaAnimation::Play(play), .. }] => {
                play.get_mut().anim = custom_asset_ids::SHINY_MISSILE_ANIM;
            },
            _ => panic!(),
        }

        let mut new_ancs_bytes = vec![];
        ancs.write_to(&mut new_ancs_bytes).unwrap();

        build_resource(
            custom_asset_ids::SHINY_MISSILE_ANCS,
            structs::ResourceKind::External(new_ancs_bytes, b"ANCS".into())
        )
    };
    let shiny_missile_evnt = {
        let mut evnt = resources[&resource_info!("Missile_Launcher_ready.EVNT").into()]
            .kind.as_evnt()
            .unwrap().into_owned();


        evnt.effect_events.as_mut_vec()[0].effect_file_id = resource_info!("healthnew.PART").res_id;
        evnt.effect_events.as_mut_vec()[1].effect_file_id = resource_info!("healthnew.PART").res_id;

        build_resource(
            custom_asset_ids::SHINY_MISSILE_EVNT,
            structs::ResourceKind::Evnt(evnt)
        )
    };
    let shiny_missile_anim = {
        let shiny_missile_anim = ResourceData::new(
            &resources[&resource_info!("Missile_Launcher_ready.ANIM").into()]
        );
        let mut anim_bytes = shiny_missile_anim.decompress().into_owned();
        custom_asset_ids::SHINY_MISSILE_EVNT.write_to(&mut std::io::Cursor::new(&mut anim_bytes[8..])).unwrap();
        build_resource(
            custom_asset_ids::SHINY_MISSILE_ANIM,
            structs::ResourceKind::External(anim_bytes, b"ANIM".into())
        )
    };
    [shiny_missile_cmdl, shiny_missile_ancs, shiny_missile_evnt, shiny_missile_anim]
}

fn create_item_scan_strg_pair<'r>(
    new_scan: ResId<res_id::SCAN>,
    new_strg: ResId<res_id::STRG>,
    content: Vec<String>
) -> [structs::Resource<'r>; 2]
{
    let scan = build_resource(
        new_scan,
        structs::ResourceKind::Scan(structs::Scan {
            frme: ResId::invalid(),
            strg: new_strg,
            scan_speed: 0,
            category: 0,
            icon_flag: 0,
            images: Default::default(),
            padding: [255; 23].into(),
            _dummy: std::marker::PhantomData,
        }),
    );

    let strg = build_resource(
        new_strg,
        structs::ResourceKind::Strg(structs::Strg::from_strings(content)),
    );

    [scan, strg]
}
