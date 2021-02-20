use resource_info_table::resource_info;
use reader_writer::{FourCC, Reader, Writable};
use structs::{res_id, ResId, Resource, ResourceKind};

use crate::{
    pickup_meta::PickupType,
    starting_items::StartingItems,
    ResourceData,
};

use std::collections::HashMap;

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

        SKIP_HUDMEMO_STRG_START: STRG,
        SKIP_HUDMEMO_STRG_END: STRG = SKIP_HUDMEMO_STRG_START.to_u32() + 38,
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
fn extra_assets<'r>() -> Vec<Resource<'r>>
{
    let extra_assets: &[((u32, FourCC), &[u8])] = &[
        // Phazon Suit TXTR 1
        (custom_asset_ids::PHAZON_SUIT_TXTR1.into(),
        include_bytes!("../extra_assets/phazon_suit_texure_1.txtr")),
        // Phazon Suit TXTR 2
        (custom_asset_ids::PHAZON_SUIT_TXTR2.into(),
        include_bytes!("../extra_assets/phazon_suit_texure_2.txtr")),
        // Nothing texture
        (custom_asset_ids::NOTHING_TXTR.into(),
        include_bytes!("../extra_assets/nothing_texture.txtr")),
        // Shiny Missile TXTR 0
        (custom_asset_ids::SHINY_MISSILE_TXTR0.into(),
        include_bytes!("../extra_assets/shiny-missile0.txtr")),
        // Shiny Missile TXTR 1
        (custom_asset_ids::SHINY_MISSILE_TXTR1.into(),
        include_bytes!("../extra_assets/shiny-missile1.txtr")),
        // Shiny Missile TXTR 2
        (custom_asset_ids::SHINY_MISSILE_TXTR2.into(),
        include_bytes!("../extra_assets/shiny-missile2.txtr")),
    ];

    extra_assets.iter().map(|&((file_id, fourcc), bytes)| {
        build_resource_raw(file_id, ResourceKind::Unknown(Reader::new(bytes), fourcc))
    }).collect()
}

pub fn custom_assets<'r>(
    resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
    starting_items: &StartingItems
) -> Vec<Resource<'r>>
{
    let mut assets = extra_assets();
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
        "Phazon Suit\0",
    ));
    assets.extend_from_slice(&create_item_scan_strg_pair(
        custom_asset_ids::NOTHING_SCAN,
        custom_asset_ids::NOTHING_SCAN_STRG,
        "???\0",
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
        "Thermal Visor\0",
    ));
    assets.extend_from_slice(&create_item_scan_strg_pair(
        custom_asset_ids::SCAN_VISOR_SCAN,
        custom_asset_ids::SCAN_VISOR_SCAN_STRG,
        "Scan Visor\0",
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
        "Shiny Missile\0",
    ));
    assets.extend_from_slice(&create_shiny_missile_assets(resources));
    assets.push(build_resource(
        custom_asset_ids::SHINY_MISSILE_ACQUIRED_HUDMEMO_STRG,
        structs::ResourceKind::Strg(structs::Strg::from_strings(vec![
            "&just=center;Shiny Missile acquired!\0".to_owned(),
        ])),
    ));
    assets.push(build_resource(
        custom_asset_ids::STARTING_ITEMS_HUDMEMO_STRG,
        structs::ResourceKind::Strg(create_starting_items_hud_memo_strg(starting_items)),
    ));

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

    assets
}

fn create_starting_items_hud_memo_strg<'r>(starting_items: &StartingItems) -> structs::Strg<'r>
{
    let mut items = vec![];

    if starting_items.scan_visor {
        items.push("Scan Visor");
    }

    let missiles_text: String;
    if starting_items.missiles > 1 {
        missiles_text = format!("{} Missiles", starting_items.missiles);
        items.push(&missiles_text[..]);
    }

    let energy_tanks_text: String;
    if starting_items.energy_tanks >= 1 {
        let text = if starting_items.energy_tanks == 1 {
            "1 Energy Tank"
        } else {
            energy_tanks_text = format!("{} Energy Tanks", starting_items.energy_tanks);
            &energy_tanks_text[..]
        };
        items.push(text);
    }

    let power_bombs_text: String;
    if starting_items.power_bombs >= 1 {
        let text = if starting_items.power_bombs == 1 {
            "1 Power Bomb"
        } else {
            power_bombs_text = format!("{} Power Bombs", starting_items.power_bombs);
            &power_bombs_text
        };
        items.push(text);
    }

    if starting_items.wave {
        items.push("Wave Beam");
    }
    if starting_items.ice {
        items.push("Ice Beam");
    }
    if starting_items.plasma {
        items.push("Plasma Beam");
    }
    if starting_items.charge {
        items.push("Charge Beam");
    }
    if starting_items.morph_ball {
        items.push("Morph Ball");
    }
    if starting_items.bombs {
        items.push("Morph Ball Bombs");
    }
    if starting_items.spider_ball {
        items.push("Spider Ball");
    }
    if starting_items.boost_ball {
        items.push("Boost Ball");
    }
    if starting_items.varia_suit {
        items.push("Varia Suit");
    }
    if starting_items.gravity_suit {
        items.push("Gravity Suit");
    }
    if starting_items.phazon_suit {
        items.push("Phazon Suit");
    }
    if starting_items.thermal_visor {
        items.push("Thermal Visor");
    }
    if starting_items.xray {
        items.push("XRay Visor");
    }
    if starting_items.space_jump {
        items.push("Space Jump Boots");
    }
    if starting_items.grapple {
        items.push("Grapple Beam");
    }
    if starting_items.super_missile {
        items.push("Super Missile");
    }
    if starting_items.wavebuster {
        items.push("Wavebuster");
    }
    if starting_items.ice_spreader {
        items.push("Ice Spreader");
    }
    if starting_items.flamethrower {
        items.push("Flamethrower");
    }

    let mut items_arr = vec![];
    for (i, item) in items.chunks(11).enumerate() {
        if i == 0 {
            items_arr.push(format!("&just=center;Additional Starting Items : {}\0", item.join(", ")).to_owned());
        } else {
            items_arr.push(format!("&just=center;{}\0", item.join(", ")).to_owned());
        }
    }

    structs::Strg::from_strings(items_arr)
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
    contents: &str,
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
        structs::ResourceKind::Strg(structs::Strg::from_strings(vec![contents.to_owned()])),
    );
    [scan, strg]
}
