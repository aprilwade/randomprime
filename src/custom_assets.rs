use resource_info_table::resource_info;
use reader_writer::{
    FourCC,
    Reader,
    Writable,
};
use structs::{res_id, ResId, Resource, ResourceKind};

use crate::{
    patch_config::PatchConfig,
    elevators::{World, SpawnRoomData},
    pickup_meta::{self, PickupType, PickupModel},
    door_meta::{DoorType, BlastShieldType},
    ResourceData,
    GcDiscLookupExtensions,
};

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct PickupHashKey {
    pub level_id: u32,
    pub room_id: u32,
    pub pickup_idx: u32,
}

impl PickupHashKey {
    fn from_location(level_name: &str, room_name: &str, pickup_idx: u32) -> Self
    {
        let level = World::from_json_key(level_name);
        PickupHashKey {
            level_id: level.mlvl(),
            room_id: SpawnRoomData::from_str(&format!("{}:{}", level.to_str(), room_name).as_str()).mrea, // TODO: this is suboptimal
            pickup_idx,
        }
    }
}

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
        PHAZON_SUIT_TXTR1: TXTR = 0xDEAF0000,
        PHAZON_SUIT_TXTR2: TXTR,
        PHAZON_SUIT_CMDL: CMDL,
        PHAZON_SUIT_ANCS: ANCS,
        NOTHING_TXTR: TXTR,
        NOTHING_CMDL: CMDL,
        NOTHING_ANCS: ANCS,
        SHINY_MISSILE_TXTR0: TXTR,
        SHINY_MISSILE_TXTR1: TXTR,
        SHINY_MISSILE_TXTR2: TXTR,
        SHINY_MISSILE_CMDL: CMDL,
        SHINY_MISSILE_ANCS: ANCS,
        SHINY_MISSILE_EVNT: EVNT,
        SHINY_MISSILE_ANIM: ANIM,
        SHORELINES_POI_SCAN: SCAN,
        SHORELINES_POI_STRG: STRG,
        MQA_POI_SCAN: SCAN,
        MQA_POI_STRG: STRG,
        CFLDG_POI_SCAN: SCAN,
        CFLDG_POI_STRG: STRG,
        
        // Starting items memo
        STARTING_ITEMS_HUDMEMO_STRG: STRG,
        
        // Warping to start transition message
        WARPING_TO_START_STRG: STRG,

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
        MAP_DOT_TXTR: TXTR,

        // Strings to use if none are specified
        DEFAULT_PICKUP_SCAN_STRGS: STRG,
        DEFAULT_PICKUP_SCANS: SCAN = DEFAULT_PICKUP_SCAN_STRGS.to_u32() + 50,
        DEFAULT_PICKUP_HUDMEMO_STRGS: STRG = DEFAULT_PICKUP_SCANS.to_u32() + 50,

        EXTRA_IDS_START: STRG = DEFAULT_PICKUP_HUDMEMO_STRGS.to_u32() + 50,
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
        (custom_asset_ids::MAP_DOT_TXTR,              *b"TXTR", include_bytes!("../extra_assets/map_pickupdot.txtr")),
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
    pickup_hudmemos: &mut HashMap::<PickupHashKey, ResId<res_id::STRG>>,
    pickup_scans: &mut HashMap<PickupHashKey, (ResId<res_id::SCAN>, ResId<res_id::STRG>)>,
    extra_scans: &mut HashMap<PickupHashKey, (ResId<res_id::SCAN>, ResId<res_id::STRG>)>,
    config: &PatchConfig,
) -> (Vec<Resource<'r>>, Vec<ResId<res_id::SCAN>>)
{
    /*  This is a list of all custom SCAN IDs which might be used throughout the game.
        We need to patch these into a SAVW file so that the game engine allocates enough space
        on initialization to store each individual scan's completion %.
    */
    let mut savw_scans_to_add: Vec<ResId<res_id::SCAN>> = Vec::new();

    // External assets
    let mut assets = extern_assets();

    // Custom pickup model assets
    assets.extend_from_slice(&create_nothing_icon_cmdl_and_ancs(
        resources,
        custom_asset_ids::NOTHING_CMDL,
        custom_asset_ids::NOTHING_ANCS,
        //ResId::<res_id::TXTR>::new(0xBE4CD99D),
        custom_asset_ids::NOTHING_TXTR,
        ResId::<res_id::TXTR>::new(0xF68DF7F1),
    ));
    assets.extend_from_slice(&create_suit_icon_cmdl_and_ancs(
        resources,
        custom_asset_ids::PHAZON_SUIT_CMDL,
        custom_asset_ids::PHAZON_SUIT_ANCS,
        custom_asset_ids::PHAZON_SUIT_TXTR1,
        custom_asset_ids::PHAZON_SUIT_TXTR2,
    ));
    assets.extend_from_slice(&create_shiny_missile_assets(resources));
    assets.extend_from_slice(&create_item_scan_strg_pair(
        custom_asset_ids::SHORELINES_POI_SCAN,
        custom_asset_ids::SHORELINES_POI_STRG,
        "task failed successfully\0",
    ));
    savw_scans_to_add.push(custom_asset_ids::SHORELINES_POI_SCAN);
    assets.extend_from_slice(&create_item_scan_strg_pair(
        custom_asset_ids::MQA_POI_SCAN,
        custom_asset_ids::MQA_POI_STRG,
        "Scan Visor is a Movement System.\0",
    ));
    savw_scans_to_add.push(custom_asset_ids::MQA_POI_SCAN);
    assets.extend_from_slice(&create_item_scan_strg_pair(
        custom_asset_ids::CFLDG_POI_SCAN,
        custom_asset_ids::CFLDG_POI_STRG,
        "Toaster's Champions: Awp82\0",
    ));
    savw_scans_to_add.push(custom_asset_ids::CFLDG_POI_SCAN);

    if starting_memo.is_some() {
        assets.push(build_resource(
            custom_asset_ids::STARTING_ITEMS_HUDMEMO_STRG,
            structs::ResourceKind::Strg(structs::Strg::from_strings(vec![
                format!("&just=center;{}\0", starting_memo.clone().unwrap()),
            ])),
        ));
    }

    // Create fallback/default scan/scan-text/hudmemo assets //
    for pt in PickupType::iter() {
        let name: &str = pt.name();
        assets.extend_from_slice(&create_item_scan_strg_pair(
            pt.scan(),
            pt.scan_strg(),
            &format!("{}\0", name),
        ));
        savw_scans_to_add.push(pt.scan());

        assets.push(build_resource(
            pt.hudmemo_strg(),
            structs::ResourceKind::Strg(structs::Strg::from_strings(vec![
                format!("&just=center;{} aquired\0", name),
            ])),
        ));
    }

    // Create user-defined hudmemo and scan strings and map to locations //
    let mut custom_asset_offset = 0;
    for (level_name, level) in config.level_data.iter() {
        for (room_name, room) in level.rooms.iter() {
            let mut pickup_idx = 0;
            let mut extra_scans_idx = 0;

            if room.extra_scans.is_some() {
                for custom_scan in room.extra_scans.as_ref().unwrap().iter() {
                    // Get next 2 IDs //
                    let scan_id = ResId::<res_id::SCAN>::new(custom_asset_ids::EXTRA_IDS_START.to_u32() + custom_asset_offset);
                    custom_asset_offset = custom_asset_offset + 1;
                    let strg_id = ResId::<res_id::STRG>::new(custom_asset_ids::EXTRA_IDS_START.to_u32() + custom_asset_offset);
                    custom_asset_offset = custom_asset_offset + 1;

                    let is_red = {
                        if custom_scan.is_red {
                            1
                        } else {
                            0
                        }
                    };

                    assets.extend_from_slice(&create_item_scan_strg_pair_2(
                        scan_id,
                        strg_id,
                        format!("{}\0", custom_scan.text).as_str(),
                        is_red,
                    ));

                    // Map for easy lookup when patching //
                    let key = PickupHashKey::from_location(level_name, room_name, extra_scans_idx);
                    extra_scans.insert(key, (scan_id, strg_id));
                    savw_scans_to_add.push(scan_id);

                    extra_scans_idx = extra_scans_idx + 1;
                }
            }

            if room.pickups.is_none() { continue };
            for pickup in room.pickups.as_ref().unwrap().iter() {
                // custom hudmemo string
                if pickup.hudmemo_text.is_some()
                {
                    let hudmemo_text = pickup.hudmemo_text.as_ref().unwrap();

                    // Get next ID //
                    let strg_id = ResId::<res_id::STRG>::new(custom_asset_ids::EXTRA_IDS_START.to_u32() + custom_asset_offset);
                    custom_asset_offset = custom_asset_offset + 1;

                    // Build resource //
                    let strg = structs::ResourceKind::Strg(structs::Strg {
                        string_tables: vec![
                            structs::StrgStringTable {
                                lang: b"ENGL".into(),
                                strings: vec![format!("&just=center;{}\u{0}",
                                                      hudmemo_text).into()].into(),
                            },
                        ].into(),
                    });
                    let resource = build_resource(strg_id, strg);
                    assets.push(resource);
    
                    // Map for easy lookup when patching //
                    let key = PickupHashKey::from_location(level_name, room_name, pickup_idx);
                    pickup_hudmemos.insert(key, strg_id);
                }

                // Custom scan string
                if pickup.scan_text.is_some()
                {
                    let scan_text = pickup.scan_text.as_ref().unwrap();

                    // Get next 2 IDs //
                    let scan_id = ResId::<res_id::SCAN>::new(custom_asset_ids::EXTRA_IDS_START.to_u32() + custom_asset_offset);
                    custom_asset_offset = custom_asset_offset + 1;
                    let strg_id = ResId::<res_id::STRG>::new(custom_asset_ids::EXTRA_IDS_START.to_u32() + custom_asset_offset);
                    custom_asset_offset = custom_asset_offset + 1;

                    // Build resource //
                    if room_name.trim().to_lowercase() == "research core" // make the research core scan red because it goes on the terminal
                    {
                        assets.extend_from_slice(&create_item_scan_strg_pair_2(
                            scan_id,
                            strg_id,
                            format!("{}\0", scan_text).as_str(),
                            1,
                        ));
                    }
                    else
                    {
                        assets.extend_from_slice(&create_item_scan_strg_pair(
                            scan_id,
                            strg_id,
                            format!("{}\0", scan_text).as_str(),
                        ));
                    }

                    // Map for easy lookup when patching //
                    let key = PickupHashKey::from_location(level_name, room_name, pickup_idx);
                    pickup_scans.insert(key, (scan_id, strg_id));
                    savw_scans_to_add.push(scan_id);
                }

                pickup_idx = pickup_idx + 1;
            }
        }
    }
    
    // Warping to starting area
    assets.push(build_resource(
        custom_asset_ids::WARPING_TO_START_STRG,
        structs::ResourceKind::Strg(structs::Strg::from_strings(vec![
            "&just=center;Returning to starting room...\0".to_owned(),
        ])),
    ));

    // Custom door assets
    for door_type in DoorType::iter() {
        if door_type.shield_cmdl().to_u32() >= 0xDEAF0000 { // only if it doesn't exist in-game already
            assets.push(create_custom_door_cmdl(resources, door_type));
        }
    }

    (assets, savw_scans_to_add)
}

// When modifying resources in an MREA, we need to give the room a copy of the resources/
// assests used b. Create a cache of all the resources needed by any pickup, door, etc...
pub fn collect_game_resources<'r>(
    gc_disc: &structs::GcDisc<'r>,
    starting_memo: Option<&str>,
    config: &PatchConfig,
)
    -> (
        HashMap<(u32, FourCC), structs::Resource<'r>>,
        HashMap<PickupHashKey, ResId<res_id::STRG>>,
        HashMap<PickupHashKey, (ResId<res_id::SCAN>, ResId<res_id::STRG>)>,
        HashMap<PickupHashKey, (ResId<res_id::SCAN>, ResId<res_id::STRG>)>,
        Vec<ResId<res_id::SCAN>>,
    )
{
    // Get list of all dependencies patcher needs //
    let mut looking_for = HashSet::<_>::new();
    looking_for.extend(PickupModel::iter().flat_map(|x| x.dependencies().iter().cloned()));
    looking_for.extend(DoorType::iter().flat_map(|x| x.dependencies()));
    looking_for.extend(BlastShieldType::iter().flat_map(|x| x.dependencies()));

    let mut deps: Vec<(u32, FourCC)> = Vec::new();
    deps.push((0xDCEC3E77,FourCC::from_bytes(b"FRME")));
    looking_for.extend(deps);

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
                assert!(found.insert(key, res.into_owned()).is_none());
            }
        }
    }

    // Maps pickup location to STRG to use
    let mut pickup_hudmemos = HashMap::<PickupHashKey, ResId<res_id::STRG>>::new();
    let mut pickup_scans = HashMap::<PickupHashKey, (ResId<res_id::SCAN>, ResId<res_id::STRG>)>::new();
    let mut extra_scans = HashMap::<PickupHashKey, (ResId<res_id::SCAN>, ResId<res_id::STRG>)>::new();

    // Remove extra assets from dependency search since they won't appear     //
    // in any pak. Instead add them to the output resource pool. These assets //
    // are provided as external files checked into the repository.            //
    let (custom_assets, savw_scans_to_add) = custom_assets(&found, starting_memo, &mut pickup_hudmemos, &mut pickup_scans, &mut extra_scans, config);
    for res in custom_assets {
        let key = (res.file_id, res.fourcc());
        looking_for.remove(&key);
        found.insert(key, res);
    }

    if !looking_for.is_empty() {
        panic!("error - still looking for {:?}", looking_for);
    }

    (found, pickup_hudmemos, pickup_scans, extra_scans, savw_scans_to_add)
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

fn create_nothing_icon_cmdl_and_ancs<'r>(
    resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
    new_cmdl_id: ResId<res_id::CMDL>,
    new_ancs_id: ResId<res_id::ANCS>,
    new_txtr1: ResId<res_id::TXTR>,
    _new_txtr2: ResId<res_id::TXTR>,
) -> [structs::Resource<'r>; 2]
{
    let new_suit_cmdl = {
        let grav_suit_cmdl = ResourceData::new(
            &resources[&resource_info!("Metroid.CMDL").into()]
        );
        let cmdl_bytes = grav_suit_cmdl.decompress().into_owned();
        let mut cmdl: structs::Cmdl = Reader::new(&cmdl_bytes[..]).read::<structs::Cmdl>(());

        cmdl.material_sets.as_mut_vec()[0].texture_ids.as_mut_vec()[0] = new_txtr1;
        cmdl.material_sets.as_mut_vec()[0].texture_ids.as_mut_vec()[1] = new_txtr1;
        cmdl.material_sets.as_mut_vec()[0].texture_ids.as_mut_vec()[2] = new_txtr1;
        cmdl.material_sets.as_mut_vec()[0].texture_ids.as_mut_vec()[3] = new_txtr1;
        cmdl.material_sets.as_mut_vec()[0].texture_ids.as_mut_vec()[4] = new_txtr1;
        cmdl.material_sets.as_mut_vec()[0].texture_ids.as_mut_vec()[5] = new_txtr1;
        cmdl.material_sets.as_mut_vec()[0].texture_ids.as_mut_vec()[6] = new_txtr1;
        cmdl.material_sets.as_mut_vec()[0].texture_ids.as_mut_vec()[7] = new_txtr1;

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
    contents: &str,
) -> [structs::Resource<'r>; 2]
{
    create_item_scan_strg_pair_2(new_scan, new_strg, contents, 0)
}

fn create_item_scan_strg_pair_2<'r>(
    new_scan: ResId<res_id::SCAN>,
    new_strg: ResId<res_id::STRG>,
    contents: &str,
    is_important: u8,
) -> [structs::Resource<'r>; 2]
{
    let scan = build_resource(
        new_scan,
        structs::ResourceKind::Scan(structs::Scan {
            frme: ResId::<res_id::FRME>::new(0xDCEC3E77),
            strg: new_strg,
            scan_speed: 0,
            category: 0,
            icon_flag: is_important,
            images: [
                structs::ScanImage {
                    txtr: ResId::invalid(),
                    appearance_percent: 0.25,
                    image_position: 0xFFFFFFFF,
                    width: 0,
                    height: 0,
                    interval: 0.0,
                    fade_duration: 0.0,
                },
                structs::ScanImage {
                    txtr: ResId::invalid(),
                    appearance_percent: 0.50,
                    image_position: 0xFFFFFFFF,
                    width: 0,
                    height: 0,
                    interval: 0.0,
                    fade_duration: 0.0,
                },
                structs::ScanImage {
                    txtr: ResId::invalid(),
                    appearance_percent: 0.75,
                    image_position: 0xFFFFFFFF,
                    width: 0,
                    height: 0,
                    interval: 0.0,
                    fade_duration: 0.0,
                },
                structs::ScanImage {
                    txtr: ResId::invalid(),
                    appearance_percent: 1.0,
                    image_position: 0xFFFFFFFF,
                    width: 0,
                    height: 0,
                    interval: 0.0,
                    fade_duration: 0.0,
                },
            ].into(),
            padding: [255; 23].into(),
            _dummy: std::marker::PhantomData,
        }),
    );
    let strg = build_resource(
        new_strg,
        structs::ResourceKind::Strg(structs::Strg::from_strings(vec![contents.to_owned()])),
    );
    [scan, strg]
}
