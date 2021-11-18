use encoding::{
    all::WINDOWS_1252,
    Encoding,
    EncoderTrap,
};

use rand::{
    rngs::StdRng,
    seq::SliceRandom,
    SeedableRng,
    Rng,
};

use crate::patch_config::{
    ArtifactHintBehavior,
    MapState,
    IsoFormat,
    PickupConfig,
    PatchConfig,
    GameBanner,
    LevelConfig,
    CtwkConfig,
    CutsceneMode,
    DoorConfig,
};

use crate::{
    custom_assets::{custom_asset_ids, collect_game_resources, PickupHashKey},
    dol_patcher::DolPatcher,
    ciso_writer::CisoWriter,
    elevators::{Elevator, SpawnRoom, SpawnRoomData, World, is_elevator},
    gcz_writer::GczWriter,
    mlvl_wrapper,
    pickup_meta::{self, PickupType, PickupModel, DoorLocation},
    door_meta::{DoorType},
    patcher::{PatcherState, PrimePatcher},
    starting_items::StartingItems,
    txtr_conversions::{
        cmpr_compress,
        cmpr_decompress,
        huerotate_in_place,
        VARIA_SUIT_TEXTURES,
        PHAZON_SUIT_TEXTURES,
    },
    GcDiscLookupExtensions,
    extern_assets::ExternPickupModel,
};

use dol_symbol_table::mp1_symbol;
use resource_info_table::{resource_info, ResourceInfo};
use ppcasm::ppcasm;

use reader_writer::{
    generic_array::GenericArray,
    typenum::U3,
    CStrConversionExtension,
    FourCC,
    Reader,
    Writable,
};
use structs::{res_id, ResId};

use std::{
    borrow::Cow,
    collections::HashMap,
    convert::TryInto,
    ffi::CString,
    fmt,
    io::Write,
    iter,
    mem,
};

const ARTIFACT_OF_TRUTH_REQ_LAYER: u32 = 24;

fn artifact_layer_change_template<'r>(instance_id: u32, pickup_kind: u32)
    -> structs::SclyObject<'r>
{
    let layer = if pickup_kind > 29 {
        pickup_kind - 28
    } else {
        assert!(pickup_kind == 29);
        ARTIFACT_OF_TRUTH_REQ_LAYER
    };
    structs::SclyObject {
        instance_id,
        connections: vec![].into(),
        property_data: structs::SpecialFunction::layer_change_fn(
            b"Artifact Layer Switch\0".as_cstr(),
            0xCD2B0EA2,
            layer
        ).into(),
    }
}

fn post_pickup_relay_template<'r>(instance_id: u32, connections: &'static [structs::Connection])
    -> structs::SclyObject<'r>
{
    structs::SclyObject {
        instance_id,
        connections: connections.to_owned().into(),
        property_data: structs::Relay {
            name: b"Randomizer Post Pickup Relay\0".as_cstr(),
            active: 1,
        }.into(),
    }
}

fn build_artifact_temple_totem_scan_strings<R>(
    config: &PatchConfig,
    rng: &mut R,
    artifact_hints: Option<HashMap<String,String>>,
    
)
    -> [String; 12]
    where R: Rng
{
    let mut generic_text_templates = [
        "I mean, maybe it'll be in &push;&main-color=#43CD80;{room}&pop;. I forgot, to be honest.\0",
        "I'm not sure where the artifact exactly is, but like, you can try &push;&main-color=#43CD80;{room}&pop;.\0",
        "Hey man, some of the Chozo are telling me that there might be a thing in &push;&main-color=#43CD80;{room}&pop;. Just sayin'.\0",
        "Uhh umm... Where was it...? Uhhh, errr, it's definitely in &push;&main-color=#43CD80;{room}&pop;! I am 100% not totally making it up...\0",
        "Some say it may be in &push;&main-color=#43CD80;{room}&pop;. Others say that you have no business here. Please leave me alone.\0",
        "A buddy and I were drinking and thought 'Hey, wouldn't be crazy if we put it in &push;&main-color=#43CD80;{room}&pop;?' It took both of us just to put it there!\0",
        "So, uhhh, I kind of got lazy and just dropped mine somewhere... Maybe it's in the &push;&main-color=#43CD80;{room}&pop;? Who knows.\0",
        "I was super late and someone had to cover for me. She said she put it in &push;&main-color=#43CD80;{room}&pop;, so you'll just have to trust her.\0",
        "Okay, so this jerk forgets to hide his so I had to hide two. This is literally saving the planet. Anyways, mine is in &push;&main-color=#43CD80;{room}&pop;.\0",
        "To be honest, I don't really remember. I think it was... um... yeah we'll just go with that: It was &push;&main-color=#43CD80;{room}&pop;.\0",
        "Hear the words of Oh Leer, last Chozo of the Artifact Temple. May they serve you... Alright, whatever. It's in &push;&main-color=#43CD80;{room}&pop;.\0",
        "I kind of just played Frisbee with mine. It flew too far and I didn't see where it landed. Somewhere in &push;&main-color=#43CD80;{room}&pop;.\0",
    ];
    generic_text_templates.shuffle(rng);
    let mut generic_templates_iter = generic_text_templates.iter();

    // Where are the artifacts?
    let mut artifact_locations = Vec::<(&str, PickupType)>::new();
    for (_, level) in config.level_data.iter() {
        for (room_name, room) in level.rooms.iter() {
            if room.pickups.is_none() { continue };
            for pickup in room.pickups.as_ref().unwrap().iter() {
                let pickup_type = PickupType::from_str(&pickup.pickup_type);
                if pickup_type.kind() >= PickupType::ArtifactOfLifegiver.kind() && pickup_type.kind() <= PickupType::ArtifactOfStrength.kind() {
                    artifact_locations.push((&room_name.as_str(), pickup_type));
                }
            }
        }
    }

    // TODO: If there end up being a large number of these, we could use a binary search
    //       instead of searching linearly.
    // XXX It would be nice if we didn't have to use Vec here and could allocated on the stack
    //     instead, but there doesn't seem to be a way to do it that isn't extremely painful or
    //     relies on unsafe code.
    let mut specific_room_templates = [
        ("Artifact Temple", vec!["{pickup} awaits those who truly seek it.\0"]),
    ];
    for rt in &mut specific_room_templates {
        rt.1.shuffle(rng);
    }

    let mut scan_text = [
        String::new(), String::new(), String::new(), String::new(),
        String::new(), String::new(), String::new(), String::new(),
        String::new(), String::new(), String::new(), String::new(),
    ];

    // Shame there isn't a way to flatten tuples automatically
    for (room_name, pt) in artifact_locations.iter() {
        let artifact_id = (pt.kind() - PickupType::ArtifactOfLifegiver.kind()) as usize;
        if scan_text[artifact_id].len() != 0 {
            // If there are multiple of this particular artifact, then we use the first instance
            // for the location of the artifact.
            continue;
        }

        // If there are specific messages for this room, choose one, otherwise choose a generic
        // message.
        let template = specific_room_templates.iter_mut()
            .find(|row| &row.0 == room_name)
            .and_then(|row| row.1.pop())
            .unwrap_or_else(|| generic_templates_iter.next().unwrap());
        let pickup_name = pt.name();
        scan_text[artifact_id] = template.replace("{room}", room_name).replace("{pickup}", pickup_name);
    }

    // Set a default value for any artifacts that we didn't find.
    for i in 0..scan_text.len() {
        if scan_text[i].len() == 0 {
            scan_text[i] = "Artifact not present. This layout may not be completable.\0".to_owned();
        }
    }

    if artifact_hints.is_some() {
        for (artifact_name, hint) in artifact_hints.unwrap() {
            let words: Vec<&str> = artifact_name.split(" ").collect();
            let lastword = words[words.len() - 1];
            let idx = match lastword.trim().to_lowercase().as_str() {
                "lifegiver" => 0,
                "wild"      => 1,
                "world"     => 2,
                "sun"       => 3,
                "elder"     => 4,
                "spirit"    => 5,
                "truth"     => 6,
                "chozo"     => 7,
                "warrior"   => 8,
                "newborn"   => 9,
                "nature"    => 10,
                "strength"  => 11,
                _ => panic!("Error - Unknown artifact - '{}'", artifact_name)
            };
            scan_text[idx] = format!("{}\0",hint.to_owned());
        }
    }

    scan_text
}

fn patch_artifact_totem_scan_strg(res: &mut structs::Resource, text: &str)
    -> Result<(), String>
{
    let strg = res.kind.as_strg_mut().unwrap();
    for st in strg.string_tables.as_mut_vec().iter_mut() {
        let strings = st.strings.as_mut_vec();
        *strings.last_mut().unwrap() = text.to_owned().into();
    }
    Ok(())
}

fn patch_save_banner_txtr(res: &mut structs::Resource)
    -> Result<(), String>
{
    const TXTR_BYTES: &[u8] = include_bytes!("../extra_assets/save_banner.txtr");
    res.compressed = false;
    res.kind = structs::ResourceKind::Unknown(Reader::new(TXTR_BYTES), b"TXTR".into());
    Ok(())
}

fn patch_thermal_conduits_damage_vulnerabilities(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];

    let thermal_conduit_damageable_trigger_obj_ids = [
        0x000F01C8, // ruined courtyard
        0x0028043F, // research core
        0x0015006C, // main ventilation shaft section b
        0x0019002C, // reactor core
        0x00190030, // reactor core
        0x0019002E, // reactor core
        0x00190029, // reactor core
        0x001A006C, // reactor core access
        0x001A006D, // reactor core access
        0x001B008E, // cargo freight lift to deck gamma
        0x001B008F, // cargo freight lift to deck gamma
        0x001B0090, // cargo freight lift to deck gamma
        0x001E01DC, // biohazard containment
        0x001E01E1, // biohazard containment
        0x001E01E0, // biohazard containment
        0x0020002A, // biotech research area 1
        0x00200030, // biotech research area 1
        0x0020002E, // biotech research area 1
        0x0002024C, // main quarry
        0x00170141, // magmoor workstation
        0x00170142, // magmoor workstation
        0x00170143, // magmoor workstation
    ];

    for obj in layer.objects.as_mut_vec().iter_mut() {
        if thermal_conduit_damageable_trigger_obj_ids.contains(&obj.instance_id) {
            let dt = obj.property_data.as_damageable_trigger_mut().unwrap();
            dt.damage_vulnerability = DoorType::Blue.vulnerability();
            dt.health_info.health = 1.0; // single power beam shot
        }
    }

    Ok(())
}

fn is_door_lock<'r>(obj: &structs::SclyObject<'r>) -> bool {
    let actor = obj.property_data.as_actor();
    
    if actor.is_none() {
        false // non-actors are never door locks
    }
    else {
        let _actor = actor.unwrap();
        _actor.cmdl == 0x5391EDB6 || _actor.cmdl == 0x6E5D6796 // door locks are indentified by their model (check for both horizontal and vertical variants)
    }
}

fn remove_door_locks(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];
    layer.objects.as_mut_vec().retain(|obj| !is_door_lock(obj));  // keep everything that isn't a door lock
    
    Ok(())
}

fn patch_morphball_hud(res: &mut structs::Resource)
    -> Result<(), String>
{
    let frme = res.kind.as_frme_mut().unwrap();
    let widget = frme.widgets.iter_mut()
        .find(|widget| widget.name == b"textpane_bombdigits\0".as_cstr())
        .unwrap();
    // Use the version of Deface18 that has more than just numerical characters for the powerbomb
    // ammo counter
    match &mut widget.kind {
        structs::FrmeWidgetKind::TextPane(textpane) => {
            textpane.font = resource_info!("Deface18B.FONT").try_into().unwrap();
            textpane.word_wrap = 0;
        }
        _ => panic!("Widget \"textpane_bombdigits\" should be a TXPN"),
    }
    widget.origin[0] -= 0.1;

    // We need to shift all of the widgets in the bomb UI left so there's
    // room for the longer powerbomb ammo counter
    const BOMB_UI_WIDGET_NAMES: &[&[u8]] = &[
        b"model_bar",
        b"model_bombbrak0",
        b"model_bombdrop0",
        b"model_bombbrak1",
        b"model_bombdrop1",
        b"model_bombbrak2",
        b"model_bombdrop2",
        b"model_bombicon",
    ];
    for widget in frme.widgets.iter_mut() {
        if !BOMB_UI_WIDGET_NAMES.contains(&widget.name.to_bytes()) {
            continue;
        }
        widget.origin[0] -= 0.325;
    }
    Ok(())
}

fn patch_add_scans_to_savw(res: &mut structs::Resource, savw_scans_to_add: &Vec<ResId<res_id::SCAN>>)
    -> Result<(), String>
{
    let savw = res.kind.as_savw_mut().unwrap();
    let scan_array = savw.scan_array.as_mut_vec();
    for scan_id in savw_scans_to_add {
        scan_array.push(structs::ScannableObject {
            scan: ResId::<res_id::SCAN>::new(scan_id.to_u32()),
            logbook_category: 0,
        });
    }

    Ok(())
}

// TODO: factor out shared code with modify_pickups_in_mrea
fn patch_add_item<'r>(
    ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    pickup_config: &PickupConfig,
    game_resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
    pickup_hudmemos: &HashMap<PickupHashKey, ResId<res_id::STRG>>,
    pickup_scans: &HashMap<PickupHashKey, (ResId<res_id::SCAN>, ResId<res_id::STRG>)>,
    pickup_hash_key: PickupHashKey,
    skip_hudmemos: bool,
    extern_models: &HashMap<String, ExternPickupModel>,
) -> Result<(), String>
{
    let room_id = area.mlvl_area.internal_id;

    // Pickup to use for game functionality //
    let pickup_type = PickupType::from_str(&pickup_config.pickup_type);

    let extern_model = if pickup_config.model.is_some() {
        extern_models.get(pickup_config.model.as_ref().unwrap())
    } else {
        None
    };

    // Pickup to use for visuals/hitbox //
    let pickup_model_type: Option<PickupModel> = {
        if pickup_config.model.is_some() {
            let model_name = pickup_config.model.as_ref().unwrap();
            let pmt = PickupModel::from_str(&model_name);
            if pmt.is_none() && !extern_model.is_some() {
                panic!("Unkown Model Type {}", model_name);
            }

            pmt // Some - Native Prime Model
                // None - External Model (e.g. Screw Attack)
        } else {
            Some(PickupModel::from_type(pickup_type)) // No model specified, use pickup type as inspiration
        }
    };

    let pickup_model_type = pickup_model_type.clone().unwrap_or(PickupModel::Nothing);
    let mut pickup_model_data = pickup_model_type.pickup_data();
    if extern_model.is_some() {
        let scale = extern_model.as_ref().unwrap().scale.clone();
        pickup_model_data.scale[0] = pickup_model_data.scale[0]*scale;
        pickup_model_data.scale[1] = pickup_model_data.scale[1]*scale;
        pickup_model_data.scale[2] = pickup_model_data.scale[2]*scale;
        pickup_model_data.cmdl = ResId::<res_id::CMDL>::new(extern_model.as_ref().unwrap().cmdl);
        pickup_model_data.ancs.file_id = ResId::<res_id::ANCS>::new(extern_model.as_ref().unwrap().ancs);
        pickup_model_data.part = ResId::invalid();
        pickup_model_data.ancs.node_index = extern_model.as_ref().unwrap().character;
        pickup_model_data.ancs.default_animation = 0;
        pickup_model_data.actor_params.xray_cmdl = ResId::invalid();
        pickup_model_data.actor_params.xray_cskr = ResId::invalid();
        pickup_model_data.actor_params.thermal_cmdl = ResId::invalid();
        pickup_model_data.actor_params.thermal_cskr = ResId::invalid();
    }

    let new_layer_idx = if area.layer_flags.layer_count > 60 {
        let name = CString::new(format!(
            "Randomizer - Pickup ({:?})", pickup_model_data.name)).unwrap();
        area.add_layer(Cow::Owned(name));
        area.layer_flags.layer_count as usize - 1
    } else {
        0
    };

    // Add hudmemo string as dependency to room //
    let hudmemo_strg: ResId<res_id::STRG> = {
        if pickup_config.hudmemo_text.is_some() {
            *pickup_hudmemos.get(&pickup_hash_key).unwrap()
        } else {
            pickup_type.hudmemo_strg()
        }
    };

    let hudmemo_dep: structs::Dependency = hudmemo_strg.into();
    area.add_dependencies(game_resources, new_layer_idx, iter::once(hudmemo_dep));    

    /* Add Model Dependencies */
    // Dependencies are defined externally
    if extern_model.is_some() {
        let deps = extern_model.as_ref().unwrap().dependencies.clone();
        let deps_iter = deps.iter()
            .map(|&(file_id, fourcc)| structs::Dependency {
                asset_id: file_id,
                asset_type: fourcc,
            }
        );
        area.add_dependencies(game_resources, new_layer_idx, deps_iter);
    }
    // If we aren't using an external model, use the dependencies traced by resource_tracing 
    else {
        let deps_iter = pickup_model_type
            .dependencies().iter()
            .map(|&(file_id, fourcc)| structs::Dependency {
                asset_id: file_id,
                asset_type: fourcc,
                }
            );
        area.add_dependencies(game_resources, new_layer_idx, deps_iter);
    }

    {
        let frme = ResId::<res_id::FRME>::new(0xDCEC3E77);
        let frme_dep: structs::Dependency = frme.into();
        area.add_dependencies(game_resources, new_layer_idx, iter::once(frme_dep));    
    }
    let scan_id = {
        if pickup_config.scan_text.is_some() {
            let (scan, strg) = *pickup_scans.get(&pickup_hash_key).unwrap();

            let scan_dep: structs::Dependency = scan.into();
            area.add_dependencies(game_resources, new_layer_idx, iter::once(scan_dep));

            let strg_dep: structs::Dependency = strg.into();
            area.add_dependencies(game_resources, new_layer_idx, iter::once(strg_dep));

            scan
        }
        else
        {
            let scan_dep: structs::Dependency = pickup_type.scan().into();
            area.add_dependencies(game_resources, new_layer_idx, iter::once(scan_dep));

            let strg_dep: structs::Dependency = pickup_type.scan_strg().into();
            area.add_dependencies(game_resources, new_layer_idx, iter::once(strg_dep));

            pickup_type.scan()
        }
    };

    let curr_increase = {
        if pickup_type == PickupType::Nothing {
            0
        } else {
            if pickup_config.curr_increase.is_some() {
                pickup_config.curr_increase.unwrap()
            } else {
                if pickup_type == PickupType::Missile {
                    5
                } else {
                    1
                }
            }
        }
    };
    let max_increase = {
        if pickup_type == PickupType::Nothing {
            0
        } else {
            pickup_config.max_increase.unwrap_or(curr_increase)
        }
    };
    let kind = {
        if pickup_type == PickupType::Nothing {
            PickupType::HealthRefill.kind()
        } else {
            pickup_type.kind()
        }
    };

    if pickup_config.position.is_none() {
        panic!("Position is required for additional pickup in room '0x{:X}'", pickup_hash_key.room_id);
    }
    
    // If this is the echoes missile expansion model, compensate for the Z offset
    let mut pickup_position = pickup_config.position.unwrap();
    if pickup_config.model.as_ref().unwrap_or(&"".to_string()).contains(&"MissileExpansion") {
        pickup_position[2] = pickup_position[2] - 1.2;
    }

    let mut pickup = structs::Pickup {
        // Location Pickup Data
        // "How is this pickup integrated into the room?"
        name: b"customItem\0".as_cstr(),
        position: pickup_position.into(),
        rotation: [0.0, 0.0, 0.0].into(),
        hitbox: pickup_model_data.hitbox.clone(),
        scan_offset: pickup_model_data.scan_offset.clone(),
        fade_in_timer: 0.0,
        spawn_delay: 0.0,
        disappear_timer: 0.0,
        active: 1,
        drop_rate: 100.0,

        // Type Pickup Data
        // "What does this pickup do?"
        curr_increase,
        max_increase,
        kind,

        // Model Pickup Data
        // "What does this pickup look like?"
        scale: pickup_model_data.scale.clone(),
        cmdl: pickup_model_data.cmdl.clone(),
        ancs: pickup_model_data.ancs.clone(),
        part: pickup_model_data.part.clone(),
        actor_params: pickup_model_data.actor_params.clone(),
    };

    // set the scan file id //
    pickup.actor_params.scan_params.scan = scan_id;

    let mut pickup_obj = structs::SclyObject {
        instance_id: ps.fresh_instance_id_range.next().unwrap(),
        connections: vec![].into(),
        property_data: structs::SclyProperty::Pickup(
            Box::new(pickup)
        )
    };

    let hudmemo = structs::SclyObject {
        instance_id: ps.fresh_instance_id_range.next().unwrap(),
        connections: vec![].into(),
        property_data: structs::SclyProperty::HudMemo(
            Box::new(structs::HudMemo {
                name: b"myhudmemo\0".as_cstr(),
                first_message_timer: {
                    if skip_hudmemos {
                        5.0
                    } else {
                        3.0
                    }
                },
                unknown: 1,
                memo_type: {
                    if skip_hudmemos {
                        0
                    } else {
                        1
                    }
                },
                strg: hudmemo_strg,
                active: 1,
            })
        )
    };

    // Display hudmemo when item is picked up
    pickup_obj.connections.as_mut_vec().push(
        structs::Connection {
            state: structs::ConnectionState::ARRIVED,
            message: structs::ConnectionMsg::SET_TO_ZERO,
            target_object_id: hudmemo.instance_id,
        }
    );

    // create attainment audio
    let attainment_audio = structs::SclyObject {
        instance_id: ps.fresh_instance_id_range.next().unwrap(),
        connections: vec![].into(),
        property_data: structs::SclyProperty::Sound(
            Box::new(structs::Sound { // copied from main plaza half-pipe
                name: b"mysound\0".as_cstr(),
                position: pickup_position.into(),
                rotation: [0.0,0.0,0.0].into(),
                sound_id: 117,
                active: 1,
                max_dist: 50.0,
                dist_comp: 0.2,
                start_delay: 0.0,
                min_volume: 20,
                volume: 127,
                priority: 127,
                pan: 64,
                loops: 0,
                non_emitter: 1,
                auto_start: 0,
                occlusion_test: 0,
                acoustics: 0,
                world_sfx: 0,
                allow_duplicates: 0,
                pitch: 0,
            })
        )
    };

    // Play the sound when item is picked up
    pickup_obj.connections.as_mut_vec().push(
        structs::Connection {
            state: structs::ConnectionState::ARRIVED,
            message: structs::ConnectionMsg::PLAY,
            target_object_id: attainment_audio.instance_id,
        }
    );

    // update MREA layer with new Objects
    let scly = area.mrea().scly_section_mut();
    let layers = scly.layers.as_mut_vec();

    // If this is an artifact, create and push change function
    let pickup_kind = pickup_type.kind();
    if pickup_kind >= 29 && pickup_kind <= 40 {
        let instance_id = ps.fresh_instance_id_range.next().unwrap();
        let function = artifact_layer_change_template(instance_id, pickup_kind);
        layers[new_layer_idx].objects.as_mut_vec().push(function);
        pickup_obj.connections.as_mut_vec().push(
            structs::Connection {
                state: structs::ConnectionState::ARRIVED,
                message: structs::ConnectionMsg::INCREMENT,
                target_object_id: instance_id,
            }
        );
    }

    if !pickup_config.respawn.unwrap_or(false) && new_layer_idx != 0 {
        // Create Special Function to disable layer once item is obtained
        // This is needed because otherwise the item would re-appear every
        // time the room is loaded
        let special_function = structs::SclyObject {
            instance_id: ps.fresh_instance_id_range.next().unwrap(),
            connections: vec![].into(),
            property_data: structs::SclyProperty::SpecialFunction(
                Box::new(structs::SpecialFunction {
                    name: b"myspecialfun\0".as_cstr(),
                    position: [0., 0., 0.].into(),
                    rotation: [0., 0., 0.].into(),
                    type_: 16, // layer change
                    unknown0: b"\0".as_cstr(),
                    unknown1: 0.,
                    unknown2: 0.,
                    unknown3: 0.,
                    layer_change_room_id: room_id,
                    layer_change_layer_id: new_layer_idx as u32,
                    item_id: 0,
                    unknown4: 1, // active
                    unknown5: 0.,
                    unknown6: 0xFFFFFFFF,
                    unknown7: 0xFFFFFFFF,
                    unknown8: 0xFFFFFFFF,
                })
            ),
        };

        // Activate the layer change when item is picked up
        pickup_obj.connections.as_mut_vec().push(
            structs::Connection {
                state: structs::ConnectionState::ARRIVED,
                message: structs::ConnectionMsg::DECREMENT,
                target_object_id: special_function.instance_id,
            }
        );
        
        layers[new_layer_idx].objects.as_mut_vec().push(special_function);
    }

    layers[new_layer_idx].objects.as_mut_vec().push(hudmemo);
    layers[new_layer_idx].objects.as_mut_vec().push(attainment_audio);
    layers[new_layer_idx].objects.as_mut_vec().push(pickup_obj);

    Ok(())
}

fn patch_add_poi<'r>(
    ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    game_resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
    scan_id: ResId<res_id::SCAN>,
    strg_id: ResId<res_id::STRG>,
    position: [f32;3],
) -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layers = scly.layers.as_mut_vec();
    layers[0].objects.as_mut_vec().push(
        structs::SclyObject {
            instance_id: ps.fresh_instance_id_range.next().unwrap(),
            connections: vec![].into(),
            property_data: structs::SclyProperty::PointOfInterest(
                Box::new(structs::PointOfInterest {
                    name: b"mypoi\0".as_cstr(),
                    position: position.into(),
                    rotation: [0.0, 0.0, 0.0].into(),
                    active: 1,
                    scan_param: structs::scly_structs::ScannableParameters {
                        scan: scan_id,
                    },
                    point_size: 12.0,
                })
            ),
        }
    );

    let frme_id = ResId::<res_id::FRME>::new(0xDCEC3E77);

    let scan_dep: structs::Dependency = scan_id.into();
    area.add_dependencies(game_resources, 0, iter::once(scan_dep));

    let strg_dep: structs::Dependency = strg_id.into();
    area.add_dependencies(game_resources, 0, iter::once(strg_dep));

    let frme_dep: structs::Dependency = frme_id.into();
    area.add_dependencies(game_resources, 0, iter::once(frme_dep));

    Ok(())
}

fn modify_pickups_in_mrea<'r>(
    ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    pickup_config: &PickupConfig,
    pickup_location: pickup_meta::PickupLocation,
    game_resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
    pickup_hudmemos: &HashMap<PickupHashKey, ResId<res_id::STRG>>,
    pickup_scans: &HashMap<PickupHashKey, (ResId<res_id::SCAN>, ResId<res_id::STRG>)>,
    pickup_hash_key: PickupHashKey,
    skip_hudmemos: bool,
    hudmemo_delay: f32,
    qol_pickup_scans: bool,
    extern_models: &HashMap<String, ExternPickupModel>,
) -> Result<(), String>
{
    // Pickup to use for game functionality //
    let pickup_type = PickupType::from_str(&pickup_config.pickup_type);

    let extern_model = if pickup_config.model.is_some() {
        extern_models.get(pickup_config.model.as_ref().unwrap())
    } else {
        None
    };

    // Pickup to use for visuals/hitbox //
    let pickup_model_type: Option<PickupModel> = {
        if pickup_config.model.is_some() {
            let model_name = pickup_config.model.as_ref().unwrap();
            let pmt = PickupModel::from_str(&model_name);
            if pmt.is_none() && !extern_model.is_some() {
                panic!("Unkown Model Type {}", model_name);
            }

            pmt // Some - Native Prime Model
                // None - External Model (e.g. Screw Attack)
        } else {
            Some(PickupModel::from_type(pickup_type)) // No model specified, use pickup type as inspiration
        }
    };

    let pickup_model_type = pickup_model_type.clone().unwrap_or(PickupModel::Nothing);
    let mut pickup_model_data = pickup_model_type.pickup_data();
    if extern_model.is_some() {
        let scale = extern_model.as_ref().unwrap().scale.clone();
        pickup_model_data.scale[0] = pickup_model_data.scale[0]*scale;
        pickup_model_data.scale[1] = pickup_model_data.scale[1]*scale;
        pickup_model_data.scale[2] = pickup_model_data.scale[2]*scale;
        pickup_model_data.cmdl = ResId::<res_id::CMDL>::new(extern_model.as_ref().unwrap().cmdl);
        pickup_model_data.ancs.file_id = ResId::<res_id::ANCS>::new(extern_model.as_ref().unwrap().ancs);
        pickup_model_data.part = ResId::invalid();
        pickup_model_data.ancs.node_index = extern_model.as_ref().unwrap().character;
        pickup_model_data.ancs.default_animation = 0;
        pickup_model_data.actor_params.xray_cmdl = ResId::invalid();
        pickup_model_data.actor_params.xray_cskr = ResId::invalid();
        pickup_model_data.actor_params.thermal_cmdl = ResId::invalid();
        pickup_model_data.actor_params.thermal_cskr = ResId::invalid();
    }

    let name = CString::new(format!(
            "Randomizer - Pickup ({:?})", pickup_type.name())).unwrap();
    area.add_layer(Cow::Owned(name));
    let new_layer_idx = area.layer_flags.layer_count as usize - 1;

    let new_layer_2_idx = new_layer_idx + 1;
    if pickup_config.respawn.unwrap_or(false) {
        let name2 = CString::new(format!(
            "Randomizer - Pickup ({:?})", pickup_type.name())).unwrap();
        area.add_layer(Cow::Owned(name2));
        area.layer_flags.flags &= !(1 << new_layer_2_idx); // layer disabled by default
    }

    // Add hudmemo string as dependency to room //
    let hudmemo_strg: ResId<res_id::STRG> = {
        if pickup_config.hudmemo_text.is_some() {
            *pickup_hudmemos.get(&pickup_hash_key).unwrap()
        } else {
            pickup_type.hudmemo_strg()
        }
    };

    let hudmemo_dep: structs::Dependency = hudmemo_strg.into();
    area.add_dependencies(game_resources, new_layer_idx, iter::once(hudmemo_dep));    

    /* Add Model Dependencies */
    // Dependencies are defined externally
    if extern_model.is_some() {
        let deps = extern_model.as_ref().unwrap().dependencies.clone();
        let deps_iter = deps.iter()
            .map(|&(file_id, fourcc)| structs::Dependency {
                asset_id: file_id,
                asset_type: fourcc,
            }
        );
        area.add_dependencies(game_resources, new_layer_idx, deps_iter);
    }
    // If we aren't using an external model, use the dependencies traced by resource_tracing
    else {
        let deps_iter = pickup_model_type
            .dependencies().iter()
            .map(|&(file_id, fourcc)| structs::Dependency {
                asset_id: file_id,
                asset_type: fourcc,
                }
            );
        area.add_dependencies(game_resources, new_layer_idx, deps_iter);
    }

    {
        let frme = ResId::<res_id::FRME>::new(0xDCEC3E77);
        let frme_dep: structs::Dependency = frme.into();
        area.add_dependencies(game_resources, new_layer_idx, iter::once(frme_dep));    
    }
    let scan_id = {
        if pickup_config.scan_text.is_some() {
            let (scan, strg) = *pickup_scans.get(&pickup_hash_key).unwrap();

            let scan_dep: structs::Dependency = scan.into();
            area.add_dependencies(game_resources, new_layer_idx, iter::once(scan_dep));

            let strg_dep: structs::Dependency = strg.into();
            area.add_dependencies(game_resources, new_layer_idx, iter::once(strg_dep));

            scan
        }
        else
        {
            let scan_dep: structs::Dependency = pickup_type.scan().into();
            area.add_dependencies(game_resources, new_layer_idx, iter::once(scan_dep));

            let strg_dep: structs::Dependency = pickup_type.scan_strg().into();
            area.add_dependencies(game_resources, new_layer_idx, iter::once(strg_dep));

            pickup_type.scan()
        }
    };

    let room_id = area.mlvl_area.internal_id;
    let scly = area.mrea().scly_section_mut();
    let layers = scly.layers.as_mut_vec();

    let mut additional_connections = Vec::new();

    // Add a post-pickup relay. This is used to support cutscene-skipping
    let instance_id = ps.fresh_instance_id_range.next().unwrap();
    let relay = post_pickup_relay_template(instance_id,
                                            pickup_location.post_pickup_relay_connections);
    
    additional_connections.push(structs::Connection {
        state: structs::ConnectionState::ARRIVED,
        message: structs::ConnectionMsg::SET_TO_ZERO,
        target_object_id: instance_id,
    });

    // If this is an artifact, insert a layer change function
    let pickup_kind = pickup_type.kind();
    if pickup_kind >= 29 && pickup_kind <= 40 {
        let instance_id = ps.fresh_instance_id_range.next().unwrap();
        let function = artifact_layer_change_template(instance_id, pickup_kind);
        layers[new_layer_idx].objects.as_mut_vec().push(function);
        additional_connections.push(structs::Connection {
            state: structs::ConnectionState::ARRIVED,
            message: structs::ConnectionMsg::INCREMENT,
            target_object_id: instance_id,
        });
    }

    if pickup_config.respawn.unwrap_or(false) {
        // add a special function that activates this pickup
        let special_function_id = ps.fresh_instance_id_range.next().unwrap();
        layers[new_layer_idx].objects.as_mut_vec().push(structs::SclyObject {
            instance_id: special_function_id,
            connections: vec![].into(),
            property_data: structs::SpecialFunction::layer_change_fn(
                b"Enable pickup\0".as_cstr(),
                room_id,
                new_layer_2_idx as u32,
            ).into(),
        });
        layers[new_layer_2_idx].objects.as_mut_vec().push(structs::SclyObject {
            instance_id: ps.fresh_instance_id_range.next().unwrap(),
            property_data: structs::Timer {
                name: b"auto-spawn pickup\0".as_cstr(),
                start_time: 0.001,
                max_random_add: 0.0,
                reset_to_zero: 0,
                start_immediately: 1,
                active: 1,
            }.into(),
            connections: vec![
                structs::Connection {
                    state: structs::ConnectionState::ZERO,
                    message: structs::ConnectionMsg::ACTIVATE,
                    target_object_id: pickup_location.location.instance_id,
                },
            ].into(),
        });
        additional_connections.push(structs::Connection {
            state: structs::ConnectionState::ARRIVED,
            message: structs::ConnectionMsg::INCREMENT,
            target_object_id: special_function_id
        });
    }

    let pickup_obj = layers[pickup_location.location.layer as usize].objects.iter_mut()
        .find(|obj| obj.instance_id == pickup_location.location.instance_id)
        .unwrap();
    let (position, scan_id_out) = update_pickup(pickup_obj, pickup_type, pickup_model_data, pickup_config, scan_id);

    if additional_connections.len() > 0 {
        pickup_obj.connections.as_mut_vec().extend_from_slice(&additional_connections);
    }

    layers[new_layer_idx].objects.as_mut_vec().push(relay);

    // find any overlapping POI that give "helpful" hints to the player and replace their scan text with the items //
    if qol_pickup_scans {
        const EXCLUDE_POI: &[u32] = &[
            0x000200AF, // main plaza tree    
            0x00190584, 0x0019039C, // research lab hydra
            0x001F025C, // mqb tank
            custom_asset_ids::MQA_POI_SCAN.to_u32(),
        ];
        for layer in layers.iter_mut() {
            for obj in layer.objects.as_mut_vec().iter_mut() {
                let obj_id = obj.instance_id&0x00FFFFFF;
    
                // Make the door in magmoor workstaion passthrough so item is scannable
                // Also the ice in ruins west
                if obj_id == 0x0017016E || obj_id == 0x0017016F || obj_id == 0x00092738
                {    
                    let actor = obj.property_data.as_actor_mut().unwrap();
                    actor.actor_params.visor_params.target_passthrough = 1;
                } else if obj.property_data.is_point_of_interest() {
                    let poi = obj.property_data.as_point_of_interest_mut().unwrap();
                    if (
                        f32::abs(poi.position[0] - position[0]) < 6.0 &&
                        f32::abs(poi.position[1] - position[1]) < 6.0 &&
                        f32::abs(poi.position[2] - position[2]) < 3.0 &&
                        !EXCLUDE_POI.contains(&obj_id) &&
                        pickup_location.location.instance_id != 0x002005EA
                       ) || (pickup_location.location.instance_id == 0x0428011c && obj_id == 0x002803CE)  // research core scan
                         || (pickup_location.location.instance_id == 0x00020176 && poi.scan_param.scan == custom_asset_ids::SHORELINES_POI_SCAN) // custom shorelines tower scan
                         || (pickup_location.location.instance_id == 600301 && poi.scan_param.scan == 0x00092837) // Ice Ruins West scan
                    {
                        poi.scan_param.scan = scan_id_out;
                    }
                }
            }
        }
    }

    let hudmemo = layers[pickup_location.hudmemo.layer as usize].objects.iter_mut()
        .find(|obj| obj.instance_id ==  pickup_location.hudmemo.instance_id)
        .unwrap();
    // The items in Watery Hall (Charge beam), Research Core (Thermal Visor), and Artifact Temple
    // (Artifact of Truth) should ys have modal hudmenus because a cutscene plays immediately
    // after each item is acquired, and the nonmodal hudmenu wouldn't properly appear.
    update_hudmemo(hudmemo, hudmemo_strg, skip_hudmemos, hudmemo_delay);

    let location = pickup_location.attainment_audio;
    let attainment_audio = layers[location.layer as usize].objects.iter_mut()
        .find(|obj| obj.instance_id ==  location.instance_id)
        .unwrap();
    update_attainment_audio(attainment_audio, pickup_type);
    Ok(())
}

fn update_pickup(
    pickup_obj: &mut structs::SclyObject,
    pickup_type: PickupType,
    pickup_model_data: structs::Pickup,
    pickup_config: &PickupConfig,
    scan_id: ResId<res_id::SCAN>,
) -> ([f32; 3], ResId<res_id::SCAN>)
{
    let pickup = pickup_obj.property_data.as_pickup_mut().unwrap();
    let mut original_pickup = pickup.clone();

    if pickup_config.position.is_some() {
        original_pickup.position = pickup_config.position.unwrap().into();
    }

    let original_aabb = pickup_meta::aabb_for_pickup_cmdl(original_pickup.cmdl).unwrap();
    let new_aabb = pickup_meta::aabb_for_pickup_cmdl(pickup_model_data.cmdl).unwrap_or(
        pickup_meta::aabb_for_pickup_cmdl(PickupModel::EnergyTank.pickup_data().cmdl).unwrap()
    );
    let original_center = calculate_center(original_aabb, original_pickup.rotation,
                                            original_pickup.scale);
    let new_center = calculate_center(new_aabb, pickup_model_data.rotation,
                                        pickup_model_data.scale);

    let curr_increase = {
        if pickup_type == PickupType::Nothing {
            0
        } else {
            if pickup_config.curr_increase.is_some() {
                pickup_config.curr_increase.unwrap()
            } else {
                if pickup_type == PickupType::Missile {
                    5
                } else {
                    1
                }
            }
        }
    };
    let max_increase = {
        if pickup_type == PickupType::Nothing {
            0
        } else {
            pickup_config.max_increase.unwrap_or(curr_increase)
        }
    };
    let kind = {
        if pickup_type == PickupType::Nothing {
            PickupType::HealthRefill.kind()
        } else {
            pickup_type.kind()
        }
    };

    // The pickup needs to be repositioned so that the center of its model
    // matches the center of the original.
    let mut position = [
        original_pickup.position[0] - (new_center[0] - original_center[0]),
        original_pickup.position[1] - (new_center[1] - original_center[1]),
        original_pickup.position[2] - (new_center[2] - original_center[2]),
    ];

    // If this is the echoes missile expansion model, compensate for the Z offset
    if pickup_config.model.as_ref().unwrap_or(&"".to_string()).contains(&"MissileExpansion") {
        position[2] = position[2] - 1.2;
    }

    *pickup = structs::Pickup {
        // Location Pickup Data
        // "How is this pickup integrated into the room?"
        name: original_pickup.name,
        position: position.into(),
        rotation: pickup_model_data.rotation.clone().into(),
        hitbox: original_pickup.hitbox,
        scan_offset: [
            original_pickup.scan_offset[0] + (new_center[0] - original_center[0]),
            original_pickup.scan_offset[1] + (new_center[1] - original_center[1]),
            original_pickup.scan_offset[2] + (new_center[2] - original_center[2]),
        ].into(),
        fade_in_timer:  original_pickup.fade_in_timer,
        spawn_delay: original_pickup.spawn_delay,
        disappear_timer: original_pickup.disappear_timer,
        active: original_pickup.active,
        drop_rate: original_pickup.drop_rate,

        // Type Pickup Data
        // "What does this pickup do?"
        curr_increase,
        max_increase,
        kind,

        // Model Pickup Data
        // "What does this pickup look like?"
        scale: pickup_model_data.scale,
        cmdl: pickup_model_data.cmdl.clone(),
        ancs: pickup_model_data.ancs.clone(),
        part: pickup_model_data.part.clone(),
        actor_params: pickup_model_data.actor_params.clone(),
    };

    // Should we use non-default scan id? //
    pickup.actor_params.scan_params.scan = scan_id;

    (position, pickup.actor_params.scan_params.scan)
}

fn update_hudmemo(
    hudmemo: &mut structs::SclyObject,
    hudmemo_strg: ResId<res_id::STRG>,
    skip_hudmemos: bool,
    hudmemo_delay: f32,
)
{
    let hudmemo = hudmemo.property_data.as_hud_memo_mut().unwrap();
    hudmemo.strg = hudmemo_strg;
    if hudmemo_delay != 0.0 {
        hudmemo.first_message_timer = hudmemo_delay;
    }

    if skip_hudmemos {
        hudmemo.memo_type = 0;
        hudmemo.first_message_timer = 5.0;
    }
}

fn update_attainment_audio(
    attainment_audio: &mut structs::SclyObject,
    pickup_type: PickupType,
)
{
    let attainment_audio = attainment_audio.property_data.as_streamed_audio_mut().unwrap();
    let bytes = pickup_type.attainment_audio_file_name().as_bytes();
    attainment_audio.audio_file_name = bytes.as_cstr();
}

fn calculate_center(aabb: [f32; 6], rotation: GenericArray<f32, U3>, scale: GenericArray<f32, U3>)
    -> [f32; 3]
{
    let start = [aabb[0], aabb[1], aabb[2]];
    let end = [aabb[3], aabb[4], aabb[5]];

    let mut position = [0.; 3];
    for i in 0..3 {
        position[i] = (start[i] + end[i]) / 2. * scale[i];
    }

    rotate(position, [rotation[0], rotation[1], rotation[2]], [0.; 3])
}

fn rotate(mut coordinate: [f32; 3], mut rotation: [f32; 3], center: [f32; 3])
    -> [f32; 3]
{
    // Shift to the origin
    for i in 0..3 {
        coordinate[i] -= center[i];
        rotation[i] = rotation[i].to_radians();
    }

    for i in 0..3 {
        let original = coordinate;
        let x = (i + 1) % 3;
        let y = (i + 2) % 3;
        coordinate[x] = original[x] * rotation[i].cos() - original[y] * rotation[i].sin();
        coordinate[y] = original[x] * rotation[i].sin() + original[y] * rotation[i].cos();
    }

    // Shift back to original position
    for i in 0..3 {
        coordinate[i] += center[i];
    }
    coordinate
}

fn patch_samus_actor_size<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    player_size: f32,
) -> Result<(), String>
{
    let mrea_id = area.mlvl_area.mrea.to_u32();
    let scly = area.mrea().scly_section_mut();
    for layer in scly.layers.as_mut_vec() {
        for obj in layer.objects.as_mut_vec() {
            if obj.property_data.is_player_actor() {
                let player_actor = obj.property_data.as_player_actor_mut().unwrap();
                player_actor.scale[0] = player_actor.scale[0]*player_size;
                player_actor.scale[1] = player_actor.scale[1]*player_size;
                player_actor.scale[2] = player_actor.scale[2]*player_size;
            }

            if mrea_id == 0xb4b41c48
            {
                if obj.property_data.is_actor()
                {
                    let actor = obj.property_data.as_actor_mut().unwrap();
                    if actor.name.to_str().unwrap().contains(&"Samus")
                    {
                        actor.scale[0] = actor.scale[0]*player_size;
                        actor.scale[1] = actor.scale[1]*player_size;
                        actor.scale[2] = actor.scale[2]*player_size;
                    }
                }

                // for the end movie, go the extra mile and tilt the cameras down
                if player_size < 0.75
                {
                    if obj.property_data.is_camera()
                    {
                        let camera = obj.property_data.as_camera_mut().unwrap();
                        let name = camera.name.to_str().unwrap().to_lowercase();
                        if name.contains(&"buttons") {
                            camera.rotation[0] = -2.0;
                        } else if name.contains(&"camera4") {
                            camera.rotation[0] = -5.0;
                        }
                    }
                    
                    if vec![0x000004AF, 0x000004A4, 0x00000461, 0x00000477, 0x00000476, 0x00000474, 0x00000479, 0x00000478, 0x00000473, 0x0000045B].contains(&(obj.instance_id&0x0000FFFF))
                    {
                        let waypoint = obj.property_data.as_waypoint_mut().unwrap();
                        waypoint.position[2] = waypoint.position[2] - 2.2;
                    }
                }
            }
        }
    }
    Ok(())
}

fn patch_elevator_actor_size<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    player_size: f32,
) -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    for layer in scly.layers.as_mut_vec().iter_mut() {
        for obj in layer.objects.as_mut_vec().iter_mut() {
            if !obj.property_data.is_world_transporter() { continue; }
            let wt = obj.property_data.as_world_transporter_mut().unwrap();
            wt.player_scale[0] = wt.player_scale[0]*player_size;
            wt.player_scale[1] = wt.player_scale[1]*player_size;
            wt.player_scale[2] = wt.player_scale[2]*player_size;
        }
    }

    Ok(())
}

fn make_elevators_patch<'a>(
    patcher: &mut PrimePatcher<'_, 'a>,
    level_data: &HashMap<String, LevelConfig>,
    auto_enabled_elevators: bool,
    player_size: f32,
    force_vanilla_layout: bool,
)
-> (bool, bool)
{
    for (pak_name, rooms) in pickup_meta::ROOM_INFO.iter() {
        for room_info in rooms.iter() {
            patcher.add_scly_patch(
                (pak_name.as_bytes(), room_info.room_id.to_u32()),
                move |ps, area| patch_elevator_actor_size(ps, area, player_size),
            );
        }
    }

    if force_vanilla_layout {
        return (false, false);
    }

    let mut skip_frigate = true;
    let mut skip_ending_cinematic = false;
    for (_, level) in level_data.iter() {
        for (elevator_name, destination_name) in level.transports.iter() {

            // special cases, handled elsewhere
            if vec!["Frigate Escape Cutscene", "Essence Dead Cutscene"].contains(&(elevator_name.as_str())) {
                continue;
            }

            let elv = Elevator::from_str(&elevator_name);
            if elv.is_none() {
                panic!("Failed to parse elevator '{}'", elevator_name);
            }
            let elv = elv.unwrap();
            let dest = SpawnRoomData::from_str(destination_name);

            if dest.mlvl == World::FrigateOrpheon.mlvl() {
                skip_frigate = false;
            }

            if dest.mrea == SpawnRoom::EndingCinematic.spawn_room_data().mrea {
                skip_ending_cinematic = true;
            }

            patcher.add_scly_patch((elv.pak_name.as_bytes(), elv.mrea), move |ps, area| {
                let scly = area.mrea().scly_section_mut();
                for layer in scly.layers.iter_mut() {
                    let obj = layer.objects.iter_mut()
                        .find(|obj| obj.instance_id == elv.scly_id);
                    if let Some(obj) = obj {
                        let wt = obj.property_data.as_world_transporter_mut().unwrap();
                        wt.mrea = ResId::new(dest.mrea);
                        wt.mlvl = ResId::new(dest.mlvl);
                        wt.volume = 0; // Turning off the wooshing sound
                    }
                }

                if auto_enabled_elevators {
                    // Auto enable the elevator
                    let layer = &mut scly.layers.as_mut_vec()[0];
                    let mr_id = layer.objects.iter()
                        .find(|obj| obj.property_data.as_memory_relay()
                            .map(|mr| mr.name == b"Memory Relay - dim scan holo\0".as_cstr())
                            .unwrap_or(false)
                        )
                        .map(|mr| mr.instance_id);

                    if let Some(mr_id) = mr_id {
                        layer.objects.as_mut_vec().push(structs::SclyObject {
                            instance_id: ps.fresh_instance_id_range.next().unwrap(),
                            property_data: structs::Timer {
                                name: b"Auto enable elevator\0".as_cstr(),

                                start_time: 0.001,
                                max_random_add: 0f32,
                                reset_to_zero: 0,
                                start_immediately: 1,
                                active: 1,
                            }.into(),
                            connections: vec![
                                structs::Connection {
                                    state: structs::ConnectionState::ZERO,
                                    message: structs::ConnectionMsg::ACTIVATE,
                                    target_object_id: mr_id,
                                },
                            ].into(),
                        });
                    }
                }

                Ok(())
            });
    
            let dest_world_name = {
                if dest.mlvl == World::FrigateOrpheon.mlvl() {
                    "Frigate"
                } else if dest.mlvl == World::TallonOverworld.mlvl() {
                    "Tallon Overworld"
                } else if dest.mlvl == World::ChozoRuins.mlvl() {
                    "Chozo Ruins"
                } else if dest.mlvl == World::MagmoorCaverns.mlvl() {
                    "Magmoor Caverns"
                } else if dest.mlvl == World::PhendranaDrifts.mlvl() {
                    "Phendrana Drifts"
                } else if dest.mlvl == World::PhazonMines.mlvl() {
                    "Phazon Mines"
                } else if dest.mlvl == World::ImpactCrater.mlvl() {
                    "Impact Crater"
                } else if dest.mlvl == 0x13d79165 {
                    "Credits"
                } else {
                    panic!("unhandled mlvl destination - {}", dest.mlvl)
                }
            };

            let mut is_dest_elev = false;
            for elv in Elevator::iter() {
                if elv.elevator_data().mrea == dest.mrea {
                    is_dest_elev = true;
                    break;
                }
            }

            let room_dest_name = {
                if dest.mlvl == 0x13d79165 {
                    "End of Game".to_string()
                } else if is_dest_elev {
                    dest.name.replace('\0', "\n")
                } else {
                    format!("{} - {}", dest_world_name, dest.name.replace('\0', "\n"))
                }
            };
            let hologram_name = {
                if dest.mlvl == 0x13d79165 {
                    "End of Game".to_string()
                } else if is_dest_elev {
                    dest.name.replace('\0', " ")
                } else {
                    format!("{} - {}", dest_world_name, dest.name.replace('\0', " "))
                }
            };
            let control_name = hologram_name.clone();

            patcher.add_resource_patch((&[elv.pak_name.as_bytes()], elv.room_strg, b"STRG".into()), move |res| {
                let string = format!("Transport to {}\u{0}", room_dest_name);
                let strg = structs::Strg::from_strings(vec![string]);
                res.kind = structs::ResourceKind::Strg(strg);
                Ok(())
            });
            patcher.add_resource_patch((&[elv.pak_name.as_bytes()], elv.hologram_strg, b"STRG".into()), move |res| {
                let string = format!(
                    "Access to &main-color=#FF3333;{} &main-color=#89D6FF;granted. Please step into the hologram.\u{0}",
                    hologram_name,
                );
                let strg = structs::Strg::from_strings(vec![string]);
                res.kind = structs::ResourceKind::Strg(strg);
                Ok(())
            });
            patcher.add_resource_patch((&[elv.pak_name.as_bytes()], elv.control_strg, b"STRG".into()), move |res| {
                let string = format!(
                    "Transport to &main-color=#FF3333;{}&main-color=#89D6FF; active.\u{0}",
                    control_name,
                );
                let strg = structs::Strg::from_strings(vec![string]);
                res.kind = structs::ResourceKind::Strg(strg);
                Ok(())
            });
        }
    }

    (skip_frigate, skip_ending_cinematic)
}

fn patch_landing_site_cutscene_triggers(
    ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
) -> Result<(), String>
{
    // XXX I'd like to do this some other way than inserting a timer to trigger
    //     the memory relay, but I couldn't figure out how to make the memory
    //     relay default to on/enabled.
    let layer = area.mrea().scly_section_mut().layers.iter_mut().next().unwrap();
    let timer_id = ps.fresh_instance_id_range.next().unwrap();
    for obj in layer.objects.iter_mut() {
        if obj.instance_id == 427 {
            obj.connections.as_mut_vec().push(structs::Connection {
                state: structs::ConnectionState::ACTIVE,
                message: structs::ConnectionMsg::DEACTIVATE,
                target_object_id: timer_id,
            });
        }
        if obj.instance_id == 221 {
            obj.property_data.as_trigger_mut().unwrap().active = 0;
        }
    }
    layer.objects.as_mut_vec().push(structs::SclyObject {
        instance_id: timer_id,
        property_data: structs::Timer {
            name: b"Cutscene fixup timer\0".as_cstr(),

            start_time: 0.001,
            max_random_add: 0f32,
            reset_to_zero: 0,
            start_immediately: 1,
            active: 1,
        }.into(),
        connections: vec![
            structs::Connection {
                state: structs::ConnectionState::ZERO,
                message: structs::ConnectionMsg::ACTIVATE,
                target_object_id: 323,// "Memory Relay Set For Load"
            },
            structs::Connection {
                state: structs::ConnectionState::ZERO,
                message: structs::ConnectionMsg::ACTIVATE,
                target_object_id: 427,// "Memory Relay Ship"
            },
            structs::Connection {
                state: structs::ConnectionState::ZERO,
                message: structs::ConnectionMsg::ACTIVATE,
                target_object_id: 484,// "Effect_BaseLights"
            },
            structs::Connection {
                state: structs::ConnectionState::ZERO,
                message: structs::ConnectionMsg::ACTIVATE,
                target_object_id: 463,// "Actor Save Station Beam"
            },
        ].into(),
    });
    Ok(())
}

fn patch_ending_scene_straight_to_credits(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
) -> Result<(), String>
{
    let layer = area.mrea().scly_section_mut().layers.iter_mut().next().unwrap();
    let trigger = layer.objects.iter_mut()
        .find(|obj| obj.instance_id == 1103) // "Trigger - Start this Beatch"
        .unwrap();
    trigger.connections.as_mut_vec().push(structs::Connection {
        state: structs::ConnectionState::ENTERED,
        message: structs::ConnectionMsg::ACTION,
        target_object_id: 1241, // "SpecialFunction-edngame"
    });
    Ok(())
}

fn patch_teleporter_destination<'r>(
    area: &mut mlvl_wrapper::MlvlArea,
    spawn_room: SpawnRoomData,
)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let wt = scly.layers.iter_mut()
        .flat_map(|layer| layer.objects.iter_mut())
        .find(|obj| obj.property_data.is_world_transporter())
        .and_then(|obj| obj.property_data.as_world_transporter_mut())
        .unwrap();
    wt.mlvl = ResId::new(spawn_room.mlvl);
    wt.mrea = ResId::new(spawn_room.mrea);
    Ok(())
}

fn fix_artifact_of_truth_requirements(
    ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    config: &PatchConfig,
) -> Result<(), String>
{
    let artifact_temple_layer_overrides = config.artifact_temple_layer_overrides.clone().unwrap_or(HashMap::new());

    // Create a new layer that will be toggled on when the Artifact of Truth is collected
    let truth_req_layer_id = area.layer_flags.layer_count;
    area.add_layer(b"Randomizer - Got Artifact 1\0".as_cstr());
    
    // What is the item at artifact temple?
    let at_pickup_kind = {
        let mut _at_pickup_kind = 0; // nothing item if unspecified
        if config.level_data.contains_key(World::TallonOverworld.to_json_key()) {
            let rooms = &config.level_data.get(World::TallonOverworld.to_json_key()).unwrap().rooms;
            if rooms.contains_key("Artifact Temple") {
                let artifact_temple_pickups = &rooms.get("Artifact Temple").unwrap().pickups;
                if artifact_temple_pickups.is_some() {
                    let artifact_temple_pickups = artifact_temple_pickups.as_ref().unwrap();
                    if artifact_temple_pickups.len() != 0 {
                        _at_pickup_kind = PickupType::from_str(&artifact_temple_pickups[0].pickup_type).kind();
                    }
                }
            }
        }
        _at_pickup_kind
    };

    for i in 0..12 {
        let layer_number = if i == 0 {
            truth_req_layer_id
        } else {
            i + 1
        };
        let kind = i + 29;

        let exists = {
            let mut _exists = false;
            for (_, level) in config.level_data.iter() {
                if _exists {break;}
                for (_, room) in level.rooms.iter() {
                    if _exists {break;}
                    if room.pickups.is_none() { continue };
                    for pickup in room.pickups.as_ref().unwrap().iter() {
                        let pickup = PickupType::from_str(&pickup.pickup_type);
                        if pickup.kind() == kind {
                            _exists = true; // this artifact is placed somewhere in this world
                            break;
                        }
                    }
                }
            }

            for (key, value) in &artifact_temple_layer_overrides {
                let artifact_name = match kind {
                    33 => "lifegiver",
                    32 => "wild",
                    38 => "world",
                    37 => "sun",
                    31 => "elder",
                    39 => "spirit",
                    29 => "truth",
                    35 => "chozo",
                    34 => "warrior",
                    40 => "newborn",
                    36 => "nature",
                    30 => "strength",
                    _ => panic!("Unhandled artifact idx - '{}'", i),
                };

                if key.to_lowercase().contains(&artifact_name) {
                    _exists = _exists || *value; // if value is true, override
                    break;
                }
            }
            _exists
        };

        if exists && at_pickup_kind != kind {
            // If the artifact exists,
            // and it is not the artifact at the Artifact Temple
            // or it's placed in another player's game (multi-world)
            // THEN mark this layer as inactive. It will be activated when the item is collected.
            area.layer_flags.flags &= !(1 << layer_number);
        } else {
            // Either the artifact doesn't exist or it does and it is in the Artifact Temple, so
            // mark this layer as active. In the former case, it needs to always be active since it
            // will never be collect and in the latter case it needs to be active so the Ridley
            // fight can start immediately if its the last artifact collected.
            area.layer_flags.flags |= 1 << layer_number;
        }
    }

    let scly = area.mrea().scly_section_mut();

    // A relay on the new layer is created and connected to "Relay Show Progress 1"
    let new_relay_instance_id = ps.fresh_instance_id_range.next().unwrap();
    let new_relay = structs::SclyObject {
        instance_id: new_relay_instance_id,
        connections: vec![
            structs::Connection {
                state: structs::ConnectionState::ZERO,
                message: structs::ConnectionMsg::SET_TO_ZERO,
                target_object_id: 1048869,
            },
        ].into(),
        property_data: structs::Relay {
            name: b"Relay Show Progress1\0".as_cstr(),
            active: 1,
        }.into(),
    };
    scly.layers.as_mut_vec()[truth_req_layer_id as usize].objects.as_mut_vec().push(new_relay);

    // An existing relay is disconnected from "Relay Show Progress 1" and connected
    // to the new relay
    let relay = scly.layers.as_mut_vec()[1].objects.iter_mut()
        .find(|i| i.instance_id == 68158836).unwrap();
    relay.connections.as_mut_vec().retain(|i| i.target_object_id != 1048869);
    relay.connections.as_mut_vec().push(structs::Connection {
        state: structs::ConnectionState::ZERO,
        message: structs::ConnectionMsg::SET_TO_ZERO,
        target_object_id: new_relay_instance_id,
    });
    Ok(())
}

fn patch_artifact_hint_availability(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    hint_behavior: ArtifactHintBehavior,
) -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    const HINT_RELAY_OBJS: &[u32] = &[
        68157732,
        68157735,
        68157738,
        68157741,
        68157744,
        68157747,
        68157750,
        68157753,
        68157756,
        68157759,
        68157762,
        68157765,
    ];
    match hint_behavior {
        ArtifactHintBehavior::Default => (),
        ArtifactHintBehavior::All => {
            // Unconditionaly connect the hint relays directly to the relay that triggers the hints
            // to conditionally.
            let obj = scly.layers.as_mut_vec()[0].objects.iter_mut()
                .find(|obj| obj.instance_id == 1048956) // "Relay One Shot Out"
                .unwrap();
            obj.connections.as_mut_vec().extend(HINT_RELAY_OBJS.iter().map(|id| {
                structs::Connection {
                    state: structs::ConnectionState::ZERO,
                    message: structs::ConnectionMsg::SET_TO_ZERO,
                    target_object_id: *id,
                }
            }));
        },
        ArtifactHintBehavior::None => {
            // Remove relays that activate artifact hint objects
            scly.layers.as_mut_vec()[1].objects.as_mut_vec()
                .retain(|obj| !HINT_RELAY_OBJS.contains(&obj.instance_id));
        },
    }
    Ok(())
}

fn patch_sun_tower_prevent_wild_before_flaahgra(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea
) -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let idx = scly.layers.as_mut_vec()[0].objects.iter_mut()
        .position(|obj| obj.instance_id == 0x001d015b)
        .unwrap();
    let sunchamber_layer_change_trigger = scly.layers.as_mut_vec()[0].objects.as_mut_vec().remove(idx);
    *scly.layers.as_mut_vec()[1].objects.as_mut_vec() = vec![sunchamber_layer_change_trigger];
    Ok(())
}


fn patch_sunchamber_prevent_wild_before_flaahgra(
    ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea
) -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let enable_sun_tower_layer_id = ps.fresh_instance_id_range.next().unwrap();
    scly.layers.as_mut_vec()[1].objects.as_mut_vec().push(structs::SclyObject {
        instance_id: enable_sun_tower_layer_id,
        connections: vec![].into(),
        property_data: structs::SpecialFunction::layer_change_fn(
            b"Enable Sun Tower Layer Change Trigger\0".as_cstr(),
            0xcf4c7aa5,
            1,
        ).into(),
    });
    let flaahgra_dead_relay = scly.layers.as_mut_vec()[1].objects.iter_mut()
        .find(|obj| obj.instance_id == 0x42500D4)
        .unwrap();
    flaahgra_dead_relay.connections.as_mut_vec().push(structs::Connection {
        state: structs::ConnectionState::ZERO,
        message: structs::ConnectionMsg::INCREMENT,
        target_object_id: enable_sun_tower_layer_id,
    });

    Ok(())
}

fn patch_essence_cinematic_skip_whitescreen(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
) -> Result<(), String>
{
    let timer_furashi_id = 0xB00E9;
    let camera_filter_key_frame_flash_id = 0xB011B;
    let timer_flashddd_id = 0xB011D;
    let special_function_cinematic_skip_id = 0xB01DC;

    let layer = area.mrea().scly_section_mut().layers.iter_mut().next().unwrap();
    let special_function_cinematic_skip_obj = layer.objects.iter_mut()
        .find(|obj| obj.instance_id == special_function_cinematic_skip_id) // "SpecialFunction Cineamtic Skip"
        .unwrap();
    special_function_cinematic_skip_obj.connections.as_mut_vec().extend_from_slice(
        &[
            structs::Connection {
                state: structs::ConnectionState::ZERO,
                message: structs::ConnectionMsg::STOP,
                target_object_id: timer_furashi_id, // "Timer - furashi"
            },
            structs::Connection {
                state: structs::ConnectionState::ZERO,
                message: structs::ConnectionMsg::DECREMENT,
                target_object_id: camera_filter_key_frame_flash_id, // "Camera Filter Keyframe Flash"
            },
            structs::Connection {
                state: structs::ConnectionState::ZERO,
                message: structs::ConnectionMsg::STOP,
                target_object_id: timer_flashddd_id, // "Timer Flashddd"
            },
        ]);
    Ok(())
}

fn patch_essence_cinematic_skip_nomusic(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
) -> Result<(), String>
{
    let streamed_audio_essence_battle_theme_id = 0xB019E;
    let special_function_cinematic_skip_id = 0xB01DC;

    let layer = area.mrea().scly_section_mut().layers.iter_mut().next().unwrap();
    layer.objects.iter_mut()
        .find(|obj| obj.instance_id == special_function_cinematic_skip_id) // "SpecialFunction Cineamtic Skip"
        .unwrap()
        .connections
        .as_mut_vec().push(
            structs::Connection {
                state: structs::ConnectionState::ZERO,
                message: structs::ConnectionMsg::PLAY,
                target_object_id: streamed_audio_essence_battle_theme_id, // "StreamedAudio Crater Metroid Prime Stage 2 SW"
            });
    Ok(())
}

fn make_elite_research_fight_prereq_patches(patcher: &mut PrimePatcher)
{
    patcher.add_scly_patch(resource_info!("03_mines.MREA").into(), |_ps, area| {
        let flags = &mut area.layer_flags.flags;
        *flags |= 1 << 1; // Turn on "3rd pass elite bustout"
        *flags &= !(1 << 5); // Turn off the "dummy elite"
        Ok(())
    });

    patcher.add_scly_patch(resource_info!("07_mines_electric.MREA").into(), |_ps, area| {
        let scly = area.mrea().scly_section_mut();
        scly.layers.as_mut_vec()[0].objects.as_mut_vec()
            .retain(|obj| obj.instance_id != 0x1B0525 && obj.instance_id != 0x1B0522);
        Ok(())
    });
}

fn patch_research_lab_hydra_barrier<'r>(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[3];

    let obj = layer.objects.as_mut_vec().iter_mut()
        .find(|obj| obj.instance_id == 202965810)
        .unwrap();
    let actor = obj.property_data.as_actor_mut().unwrap();
    actor.actor_params.visor_params.target_passthrough = 1;
    Ok(())
}

fn patch_lab_aether_cutscene_trigger(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    version: Version,
) -> Result<(), String>
{
    let layer_num = if version == Version::NtscUTrilogy || version == Version::NtscJTrilogy || version == Version::PalTrilogy || version == Version::Pal || version == Version::NtscJ {
        4
    } else {
        5
    };
    let cutscene_trigger_id = 0x330317 + (layer_num << 26);
    let scly = area.mrea().scly_section_mut();
    let trigger = scly.layers.as_mut_vec()[layer_num as usize]
        .objects.iter_mut()
        .find(|obj| obj.instance_id == cutscene_trigger_id)
        .and_then(|obj| obj.property_data.as_trigger_mut())
        .unwrap();
    trigger.active = 0;

    Ok(())
}

fn patch_research_lab_aether_exploding_wall<'r>(
    ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea
)
    -> Result<(), String>
{
    // The room we're actually patching is Research Core..
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];

    let id = ps.fresh_instance_id_range.next().unwrap();
    let obj = layer.objects.as_mut_vec().iter_mut()
        .find(|obj| obj.instance_id == 2622568)
        .unwrap();
    obj.connections.as_mut_vec().push(structs::Connection {
        state: structs::ConnectionState::ZERO,
        message: structs::ConnectionMsg::DECREMENT,
        target_object_id: id,
    });

    layer.objects.as_mut_vec().push(structs::SclyObject {
        instance_id: id,
        property_data: structs::SpecialFunction::layer_change_fn(
            b"SpecialFunction - Remove Research Lab Aether wall\0".as_cstr(),
            0x354889CE,
            3,
        ).into(),
        connections: vec![].into(),
    });
    Ok(())
}

fn patch_observatory_2nd_pass_solvablility<'r>(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[2];

    let iter = layer.objects.as_mut_vec().iter_mut()
        .filter(|obj| obj.instance_id == 0x81E0460 || obj.instance_id == 0x81E0461);
    for obj in iter {
        obj.connections.as_mut_vec().push(structs::Connection {
            state: structs::ConnectionState::DEATH_RATTLE,
            message: structs::ConnectionMsg::INCREMENT,
            target_object_id: 0x1E02EA,// Counter - dead pirates active panel
        });
    }

    Ok(())
}

fn patch_observatory_1st_pass_softlock<'r>(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    // 0x041E0001 => starting at save station will allow us to kill pirates before the lock is active
    // 0x041E0002 => doing reverse lab will allow us to kill pirates before the lock is active
    const LOCK_DOOR_TRIGGER_IDS: &[u32] = &[
                        0x041E0381,
                        0x041E0001,
                        0x041E0002,
                    ];

    let enable_lock_relay_id = 0x041E037A;

    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[1];
    layer.objects.iter_mut()
        .find(|obj| obj.instance_id == LOCK_DOOR_TRIGGER_IDS[0])
        .unwrap()
        .connections.as_mut_vec().extend_from_slice(
            &[
                structs::Connection {
                    state: structs::ConnectionState::ENTERED,
                    message: structs::ConnectionMsg::DEACTIVATE,
                    target_object_id: LOCK_DOOR_TRIGGER_IDS[1],
                },
                structs::Connection {
                    state: structs::ConnectionState::ENTERED,
                    message: structs::ConnectionMsg::DEACTIVATE,
                    target_object_id: LOCK_DOOR_TRIGGER_IDS[2],
                },
            ]
        );

    layer.objects.as_mut_vec().extend_from_slice(&[
        structs::SclyObject {
            instance_id: LOCK_DOOR_TRIGGER_IDS[1],
            property_data: structs::Trigger {
                name: b"Trigger\0".as_cstr(),
                position: [-71.301552, -941.337952, 129.976822].into(),
                scale: [10.516006, 6.079956, 7.128998].into(),
                damage_info: structs::scly_structs::DamageInfo {
                    weapon_type: 0,
                    damage: 0.0,
                    radius: 0.0,
                    knockback_power: 0.0
                },
                force: [0.0, 0.0, 0.0].into(),
                flags: 1,
                active: 1,
                deactivate_on_enter: 1,
                deactivate_on_exit: 0
            }.into(),
            connections: vec![
                structs::Connection {
                    state: structs::ConnectionState::ENTERED,
                    message: structs::ConnectionMsg::DEACTIVATE,
                    target_object_id: LOCK_DOOR_TRIGGER_IDS[0],
                },
                structs::Connection {
                    state: structs::ConnectionState::ENTERED,
                    message: structs::ConnectionMsg::DEACTIVATE,
                    target_object_id: LOCK_DOOR_TRIGGER_IDS[2],
                },
                structs::Connection {
                    state: structs::ConnectionState::ENTERED,
                    message: structs::ConnectionMsg::SET_TO_ZERO,
                    target_object_id: enable_lock_relay_id,
                },
            ].into()
        },
        structs::SclyObject {
            instance_id: LOCK_DOOR_TRIGGER_IDS[2],
            property_data: structs::Trigger {
                name: b"Trigger\0".as_cstr(),
                position: [-71.301552, -853.694336, 129.976822].into(),
                scale: [10.516006, 6.079956, 7.128998].into(),
                damage_info: structs::scly_structs::DamageInfo {
                    weapon_type: 0,
                    damage: 0.0,
                    radius: 0.0,
                    knockback_power: 0.0
                },
                force: [0.0, 0.0, 0.0].into(),
                flags: 1,
                active: 1,
                deactivate_on_enter: 1,
                deactivate_on_exit: 0
            }.into(),
            connections: vec![
                structs::Connection {
                    state: structs::ConnectionState::ENTERED,
                    message: structs::ConnectionMsg::DEACTIVATE,
                    target_object_id: LOCK_DOOR_TRIGGER_IDS[0],
                },
                structs::Connection {
                    state: structs::ConnectionState::ENTERED,
                    message: structs::ConnectionMsg::DEACTIVATE,
                    target_object_id: LOCK_DOOR_TRIGGER_IDS[1],
                },
                structs::Connection {
                    state: structs::ConnectionState::ENTERED,
                    message: structs::ConnectionMsg::SET_TO_ZERO,
                    target_object_id: enable_lock_relay_id,
                },
            ].into()
        },
    ]);

    Ok(())
}

fn patch_main_ventilation_shaft_section_b_door<'r>(
    ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea
)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];

    layer.objects.as_mut_vec().push(structs::SclyObject {
        instance_id: ps.fresh_instance_id_range.next().unwrap(),
        property_data: structs::Trigger {
            name: b"Trigger_DoorOpen-component\0".as_cstr(),
            position: [31.232622, 442.69165, -64.20529].into(),
            scale: [6.0, 17.0, 6.0].into(),
            damage_info: structs::scly_structs::DamageInfo {
                weapon_type: 0,
                damage: 0.0,
                radius: 0.0,
                knockback_power: 0.0
            },
            force: [0.0, 0.0, 0.0].into(),
            flags: 1,
            active: 1,
            deactivate_on_enter: 0,
            deactivate_on_exit: 0
        }.into(),
        connections: vec![
            structs::Connection {
                state: structs::ConnectionState::INSIDE,
                message: structs::ConnectionMsg::SET_TO_ZERO,
                target_object_id: 1376367,
            },
        ].into(),
    });
    Ok(())
}

fn make_main_plaza_locked_door_two_ways(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];

    let trigger_dooropen_id = 0x20007;
    let timer_doorclose_id = 0x20008;
    let actor_doorshield_id = 0x20004;
    let relay_unlock_id = 0x20159;
    let trigger_doorunlock_id = 0x2000F;
    let door_id = 0x20060;
    let trigger_remove_scan_target_locked_door_id = 0x202B8;
    let scan_target_locked_door_id = 0x202F4;
    let relay_notice_ineffective_weapon_id = 0x202FD;

    layer.objects.as_mut_vec().extend_from_slice(&[
        structs::SclyObject {
            instance_id: trigger_doorunlock_id,
            property_data: structs::DamageableTrigger {
                name: b"Trigger_DoorUnlock\0".as_cstr(),
                position: [152.232117, 86.451134, 24.472418].into(),
                scale: [0.25, 4.5, 4.0].into(),
                health_info: structs::scly_structs::HealthInfo {
                    health: 1.0,
                    knockback_resistance: 1.0
                },
                damage_vulnerability: structs::scly_structs::DamageVulnerability {
                    power: 1,           // Normal
                    ice: 1,             // Normal
                    wave: 1,            // Normal
                    plasma: 1,          // Normal
                    bomb: 1,            // Normal
                    power_bomb: 1,      // Normal
                    missile: 2,         // Reflect
                    boost_ball: 2,      // Reflect
                    phazon: 1,          // Normal
                    enemy_weapon0: 3,   // Immune
                    enemy_weapon1: 2,   // Reflect
                    enemy_weapon2: 2,   // Reflect
                    enemy_weapon3: 2,   // Reflect
                    unknown_weapon0: 2, // Reflect
                    unknown_weapon1: 2, // Reflect
                    unknown_weapon2: 1, // Normal
                    charged_beams: structs::scly_structs::ChargedBeams {
                        power: 1,       // Normal
                        ice: 1,         // Normal
                        wave: 1,        // Normal
                        plasma: 1,      // Normal
                        phazon: 1       // Normal
                    },
                    beam_combos: structs::scly_structs::BeamCombos {
                        power: 2,       // Reflect
                        ice: 2,         // Reflect
                        wave: 2,        // Reflect
                        plasma: 2,      // Reflect
                        phazon: 1       // Normal
                    }
                },
                unknown0: 3, // Render Side : East
                pattern_txtr0: resource_info!("testb.TXTR").try_into().unwrap(),
                pattern_txtr1: resource_info!("testb.TXTR").try_into().unwrap(),
                color_txtr: resource_info!("blue.TXTR").try_into().unwrap(),
                lock_on: 0,
                active: 1,
                visor_params: structs::scly_structs::VisorParameters {
                    unknown0: 0,
                    target_passthrough: 0,
                    visor_mask: 15 // Combat|Scan|Thermal|XRay
                }
            }.into(),
            connections: vec![
                structs::Connection {
                    state: structs::ConnectionState::REFLECTED_DAMAGE,
                    message: structs::ConnectionMsg::SET_TO_ZERO,
                    target_object_id: relay_notice_ineffective_weapon_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::DEAD,
                    message: structs::ConnectionMsg::DEACTIVATE,
                    target_object_id: actor_doorshield_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::MAX_REACHED,
                    message: structs::ConnectionMsg::ACTIVATE,
                    target_object_id: actor_doorshield_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::DEAD,
                    message: structs::ConnectionMsg::ACTIVATE,
                    target_object_id: trigger_dooropen_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::DEAD,
                    message: structs::ConnectionMsg::SET_TO_ZERO,
                    target_object_id: door_id,
                },
            ].into(),
        },

        structs::SclyObject {
            instance_id: relay_unlock_id,
            property_data: structs::Relay {
                    name: b"Relay_Unlock\0".as_cstr(),
                    active: 1,
                }.into(),
            connections: vec![
                structs::Connection {
                    state: structs::ConnectionState::ZERO,
                    message: structs::ConnectionMsg::ACTIVATE,
                    target_object_id: actor_doorshield_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::ZERO,
                    message: structs::ConnectionMsg::ACTIVATE,
                    target_object_id: trigger_doorunlock_id,
                },
            ].into(),
        },

        structs::SclyObject {
            instance_id: trigger_dooropen_id,
            property_data: structs::Trigger {
                name: b"Trigger_DoorOpen\0".as_cstr(),
                position: [149.35614, 86.567917, 26.471249].into(),
                scale: [5.0, 5.0, 8.0].into(),
                damage_info: structs::scly_structs::DamageInfo {
                    weapon_type: 0,
                    damage: 0.0,
                    radius: 0.0,
                    knockback_power: 0.0
                },
                force: [0.0, 0.0, 0.0].into(),
                flags: 1,
                active: 0,
                deactivate_on_enter: 0,
                deactivate_on_exit: 0
            }.into(),
            connections: vec![
                structs::Connection {
                    state: structs::ConnectionState::INSIDE,
                    message: structs::ConnectionMsg::OPEN,
                    target_object_id: door_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::INSIDE,
                    message: structs::ConnectionMsg::RESET_AND_START,
                    target_object_id: timer_doorclose_id,
                },
            ].into(),
        },

        structs::SclyObject {
            instance_id: actor_doorshield_id,
            property_data: structs::Actor {
                name: b"Actor_DoorShield\0".as_cstr(),
                position: [151.951187, 86.412575, 24.403177].into(),
                rotation: [0.0, 0.0, 0.0].into(),
                scale: [1.0, 1.0, 1.0].into(),
                hitbox: [0.0, 0.0, 0.0].into(),
                scan_offset: [0.0, 0.0, 0.0].into(),
                unknown1: 1.0,
                unknown2: 0.0,
                health_info: structs::scly_structs::HealthInfo {
                    health: 5.0,
                    knockback_resistance: 1.0
                },
                damage_vulnerability: structs::scly_structs::DamageVulnerability {
                    power: 1,           // Normal
                    ice: 1,             // Normal
                    wave: 1,            // Normal
                    plasma: 1,          // Normal
                    bomb: 1,            // Normal
                    power_bomb: 1,      // Normal
                    missile: 1,         // Normal
                    boost_ball: 1,      // Normal
                    phazon: 1,          // Normal
                    enemy_weapon0: 2,   // Reflect
                    enemy_weapon1: 2,   // Reflect
                    enemy_weapon2: 2,   // Reflect
                    enemy_weapon3: 2,   // Reflect
                    unknown_weapon0: 2, // Reflect
                    unknown_weapon1: 2, // Reflect
                    unknown_weapon2: 0, // Double Damage
                    charged_beams: structs::scly_structs::ChargedBeams {
                        power: 1,       // Normal
                        ice: 1,         // Normal
                        wave: 1,        // Normal
                        plasma: 1,      // Normal
                        phazon: 0       // Double Damage
                    },
                    beam_combos: structs::scly_structs::BeamCombos {
                        power: 1,       // Normal
                        ice: 1,         // Normal
                        wave: 1,        // Normal
                        plasma: 1,      // Normal
                        phazon: 0       // Double Damage
                    }
                },
                cmdl: resource_info!("blueShield_v1.CMDL").try_into().unwrap(),
                ancs: structs::scly_structs::AncsProp {
                    file_id: ResId::invalid(), // None
                    node_index: 0,
                    default_animation: 0xFFFFFFFF, // -1
                },
                actor_params: structs::scly_structs::ActorParameters {
                    light_params: structs::scly_structs::LightParameters {
                        unknown0: 1,
                        unknown1: 1.0,
                        shadow_tessellation: 0,
                        unknown2: 1.0,
                        unknown3: 20.0,
                        color: [1.0, 1.0, 1.0, 1.0].into(),
                        unknown4: 1,
                        world_lighting: 1,
                        light_recalculation: 1,
                        unknown5: [0.0, 0.0, 0.0].into(),
                        unknown6: 4,
                        unknown7: 4,
                        unknown8: 0,
                        light_layer_id: 0
                    },
                    scan_params: structs::scly_structs::ScannableParameters {
                        scan: ResId::invalid(), // None
                    },
                    xray_cmdl: ResId::invalid(), // None
                    xray_cskr: ResId::invalid(), // None
                    thermal_cmdl: ResId::invalid(), // None
                    thermal_cskr: ResId::invalid(), // None

                    unknown0: 1,
                    unknown1: 1.0,
                    unknown2: 1.0,

                    visor_params: structs::scly_structs::VisorParameters {
                        unknown0: 0,
                        target_passthrough: 0,
                        visor_mask: 15 // Combat|Scan|Thermal|XRay
                    },
                    enable_thermal_heat: 1,
                    unknown3: 0,
                    unknown4: 1,
                    unknown5: 1.0
                },
                looping: 1,
                snow: 1,
                solid: 0,
                camera_passthrough: 0,
                active: 1,
                unknown8: 0,
                unknown9: 1.0,
                unknown10: 1,
                unknown11: 0,
                unknown12: 0,
                unknown13: 0
            }.into(),
            connections: vec![].into()
        },

        structs::SclyObject {
            instance_id: timer_doorclose_id,
            property_data: structs::Timer {
                name: b"Timer_DoorClose\0".as_cstr(),
                start_time: 0.25,
                max_random_add: 0.0,
                reset_to_zero: 1,
                start_immediately: 0,
                active: 1
            }.into(),
            connections: vec![
                structs::Connection {
                    state: structs::ConnectionState::ZERO,
                    message: structs::ConnectionMsg::CLOSE,
                    target_object_id: door_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::ZERO,
                    message: structs::ConnectionMsg::DEACTIVATE,
                    target_object_id: trigger_dooropen_id,
                },
            ].into(),
        },
    ]);

    let locked_door_scan = layer.objects.as_mut_vec().iter_mut()
        .find(|obj| obj.instance_id == scan_target_locked_door_id)
        .and_then(|obj| obj.property_data.as_point_of_interest_mut())
        .unwrap();
    locked_door_scan.active = 0;
    locked_door_scan.scan_param.scan = ResId::invalid(); // None

    let locked_door = layer.objects.as_mut_vec().iter_mut()
        .find(|obj| obj.instance_id == door_id)
        .and_then(|obj| obj.property_data.as_door_mut())
        .unwrap();
    locked_door.ancs.file_id = resource_info!("newmetroiddoor.ANCS").try_into().unwrap();
    locked_door.ancs.default_animation = 2;
    locked_door.projectiles_collide = 0;

    let trigger_remove_scan_target_locked_door_and_etank = layer.objects.as_mut_vec().iter_mut()
        .find(|obj| obj.instance_id == trigger_remove_scan_target_locked_door_id)
        .and_then(|obj| obj.property_data.as_trigger_mut())
        .unwrap();
    trigger_remove_scan_target_locked_door_and_etank.active = 0;

    layer.objects.as_mut_vec().iter_mut()
        .find(|obj| obj.instance_id == door_id)
        .unwrap()
        .connections
        .as_mut_vec()
        .extend_from_slice(
            &[
                structs::Connection {
                    state: structs::ConnectionState::OPEN,
                    message: structs::ConnectionMsg::ACTIVATE,
                    target_object_id: trigger_dooropen_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::OPEN,
                    message: structs::ConnectionMsg::START,
                    target_object_id: timer_doorclose_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::CLOSED,
                    message: structs::ConnectionMsg::DEACTIVATE,
                    target_object_id: trigger_dooropen_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::OPEN,
                    message: structs::ConnectionMsg::DEACTIVATE,
                    target_object_id: trigger_doorunlock_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::OPEN,
                    message: structs::ConnectionMsg::DEACTIVATE,
                    target_object_id: actor_doorshield_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::CLOSED,
                    message: structs::ConnectionMsg::SET_TO_ZERO,
                    target_object_id: relay_unlock_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::MAX_REACHED,
                    message: structs::ConnectionMsg::DEACTIVATE,
                    target_object_id: actor_doorshield_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::MAX_REACHED,
                    message: structs::ConnectionMsg::DEACTIVATE,
                    target_object_id: trigger_doorunlock_id,
                },
            ]
        );

    Ok(())
}

fn patch_arboretum_invisible_wall(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
) -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];
    layer.objects.as_mut_vec().retain(|obj| obj.instance_id != 0x1302AA);

    Ok(())
}

fn patch_spawn_point_position<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    new_position: [f32; 3],
    relative_position: bool,
    force_default: bool,
)
-> Result<(), String>
{
    let room_id = area.mlvl_area.mrea.to_u32();
    let scly = area.mrea().scly_section_mut();
    let layer_count = scly.layers.len();
    for i in 0..layer_count {
        let layer = &mut scly.layers.as_mut_vec()[i];
        for obj in layer.objects.as_mut_vec().iter_mut() {
            if !obj.property_data.is_spawn_point() {continue;}

            let spawn_point = obj.property_data.as_spawn_point_mut().unwrap();
            if relative_position {
                spawn_point.position[0] = spawn_point.position[0] + new_position[0];
                spawn_point.position[1] = spawn_point.position[1] + new_position[1];
                spawn_point.position[2] = spawn_point.position[2] + new_position[2];
            } else {
                spawn_point.position = new_position.into();
            }

            if force_default {
                spawn_point.default_spawn = 1;
            }
        }
    }

    if room_id == 0xF517A1EA {
        // find/copy the spawn point //
        let spawn_point = scly.layers.as_mut_vec()[3].objects.as_mut_vec().iter_mut().find(|obj| obj.property_data.is_spawn_point()).unwrap().clone();
        // delete the original in the shitty layer //
        scly.layers.as_mut_vec()[3].objects.as_mut_vec().retain(|obj| !obj.property_data.is_spawn_point());
        // write the copied spawn point to the default layer //
        scly.layers.as_mut_vec()[0].objects.as_mut_vec().push(spawn_point);
    }

    Ok(())
}

fn patch_fix_pca_crash(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    // find the loading trigger and enable it
    let scly = area.mrea().scly_section_mut();
    for layer in scly.layers.as_mut_vec() {
        for obj in layer.objects.as_mut_vec() {
            if obj.property_data.is_trigger() {
                let trigger = obj.property_data.as_trigger_mut().unwrap();
                if trigger.name.to_str().unwrap().contains(&"eliteboss") {
                    trigger.active = 1;
                }
            }
        }
    }

    Ok(())
}

fn patch_backwards_lower_mines_pca(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    // remove from scripting layers
    let scly = area.mrea().scly_section_mut();
    for layer in scly.layers.as_mut_vec() {
        layer.objects.as_mut_vec().retain(|obj| !obj.property_data.is_platform());
    }

    // remove from level/area dependencies (this wasn't a necessary excercise, but it's nice to know how to do)
    let deps_to_remove: Vec<u32> = vec![
        0x744572a0, 0xBF19A105, 0x0D3BB9B1, // cmdl
        0x3cfa9c1c, 0x165B2898, // dcln
        0x122D9D74, 0x245EEA17, 0x71A63C95, 0x7351A073, 0x8229E1A3, 0xDD3931E2, // txtr
        0xBA2E99E8, 0xD03D1FF3, 0xE6D3D35E, 0x4185C16A, 0xEFE6629B, // txtr
    ];
    for dep_array in area.mlvl_area.dependencies.deps.as_mut_vec() {
        dep_array.as_mut_vec().retain(|dep| !deps_to_remove.contains(&dep.asset_id));
    }

    Ok(())
}

fn patch_backwards_lower_mines_eqa(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    for layer in scly.layers.as_mut_vec() {
        layer.objects.as_mut_vec().retain(|obj| !obj.property_data.is_platform());
    }

    Ok(())
}

fn patch_backwards_lower_mines_mqb(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[2];
    let obj = layer.objects.as_mut_vec().iter_mut()
        .find(|obj| obj.instance_id&0x00FFFFFF == 0x001F0018)
        .unwrap();
    let actor = obj.property_data.as_actor_mut().unwrap();
    actor.actor_params.visor_params.target_passthrough = 1;
    Ok(())
}

fn patch_backwards_lower_mines_mqa(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];
    let obj = layer.objects.as_mut_vec().iter_mut()
        .find(|obj| obj.instance_id&0x00FFFFFF == 0x00200214) // metriod aggro trigger
        .unwrap();
    obj.connections.as_mut_vec().push(
        structs::Connection {
            state: structs::ConnectionState::ENTERED,
            message: structs::ConnectionMsg::SET_TO_ZERO,
            target_object_id: 0x00200464, // Relay One Shot In
        },
    );
    Ok(())
}

fn patch_backwards_lower_mines_elite_control(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[1];
    let obj = layer.objects.as_mut_vec().iter_mut()
        .find(|obj| obj.instance_id&0x00FFFFFF == 0x00100086)
        .unwrap();
    let actor = obj.property_data.as_actor_mut().unwrap();
    actor.actor_params.visor_params.target_passthrough = 1;
    Ok(())
}

fn patch_main_quarry_barrier(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[4];

    let forcefield_actor_obj_id = 0x100201DA;
    let turn_off_barrier_special_function_obj_id = 0x202B5;
    let turn_off_barrier_trigger_obj_id = 0x1002044D;

    layer.objects.as_mut_vec().push(
        structs::SclyObject {
            instance_id: turn_off_barrier_trigger_obj_id,
            property_data: structs::Trigger {
                name: b"Trigger - Disable Main Quarry barrier\0".as_cstr(),
                position: [82.412056, 9.354454, 2.807631].into(),
                scale: [10.0, 5.0, 7.0].into(),
                damage_info: structs::scly_structs::DamageInfo {
                    weapon_type: 0,
                    damage: 0.0,
                    radius: 0.0,
                    knockback_power: 0.0
                },
                force: [0.0, 0.0, 0.0].into(),
                flags: 1,
                active: 1,
                deactivate_on_enter: 1,
                deactivate_on_exit: 0
            }.into(),
            connections: vec![
                structs::Connection {
                    state: structs::ConnectionState::ENTERED,
                    message: structs::ConnectionMsg::DEACTIVATE,
                    target_object_id: forcefield_actor_obj_id,
                },
                structs::Connection {
                    state: structs::ConnectionState::ENTERED,
                    message: structs::ConnectionMsg::DECREMENT,
                    target_object_id: turn_off_barrier_special_function_obj_id,
                },
            ].into(),
        }
    );

    Ok(())
}

fn patch_main_quarry_door_lock_0_02<'r>(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];
    layer.objects.as_mut_vec().retain(|obj| obj.instance_id != 132563);
    Ok(())
}

fn patch_geothermal_core_door_lock_0_02<'r>(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];
    layer.objects.as_mut_vec().retain(|obj| obj.instance_id != 1311646);
    Ok(())
}

fn patch_hive_totem_boss_trigger_0_02(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[1];
    let trigger_obj_id = 0x4240140;

    let trigger_obj = layer.objects.as_mut_vec().iter_mut()
        .find(|obj| obj.instance_id == trigger_obj_id)
        .and_then(|obj| obj.property_data.as_trigger_mut())
        .unwrap();
    trigger_obj.position = [94.571053, 301.616028, 0.344905].into();
    trigger_obj.scale = [6.052994, 24.659973, 7.878154].into();

    Ok(())
}

fn patch_ruined_courtyard_thermal_conduits(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    version: Version,
) -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];
    let thermal_conduit_damageable_trigger_obj_id = 0xF01C8;
    let thermal_conduit_actor_obj_id = 0xF01C7;
    let debris_generator_obj_id = 0xF01DD;
    let thermal_conduit_cover_actor_obj_id = 0xF01D9;

    layer.objects.as_mut_vec().iter_mut()
        .find(|obj| obj.instance_id == thermal_conduit_damageable_trigger_obj_id)
        .and_then(|obj| obj.property_data.as_damageable_trigger_mut())
        .unwrap()
        .active = 1;

    if version == Version::NtscU0_02 {
        layer.objects.as_mut_vec().iter_mut()
            .find(|obj| obj.instance_id == thermal_conduit_actor_obj_id)
            .and_then(|obj| obj.property_data.as_actor_mut())
            .unwrap()
            .active = 1;
    } else if version == Version::NtscJ || version == Version::Pal || version == Version::NtscUTrilogy || version == Version::NtscJTrilogy || version == Version::PalTrilogy {
        layer.objects.as_mut_vec().iter_mut()
            .find(|obj| obj.instance_id == debris_generator_obj_id)
            .unwrap()
            .connections
            .as_mut_vec()
            .push(
                structs::Connection {
                    state: structs::ConnectionState::ZERO,
                    message: structs::ConnectionMsg::DEACTIVATE,
                    target_object_id: thermal_conduit_cover_actor_obj_id,
                }
            );

        let flags = &mut area.layer_flags.flags;
        *flags |= 1 << 6; // Turn on "Thermal Target"
    }

    Ok(())
}

fn patch_geothermal_core_destructible_rock_pal(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];

    let platform_obj_id = 0x1403AE;
    let scan_target_platform_obj_id = 0x1403B4;
    let actor_blocker_collision_id = 0x1403B5;

    let platform_obj = layer.objects.as_mut_vec().iter_mut()
        .find(|obj| obj.instance_id == platform_obj_id)
        .and_then(|obj| obj.property_data.as_platform_mut())
        .unwrap();
    platform_obj.active = 0;

    let scan_target_platform_obj = layer.objects.as_mut_vec().iter_mut()
        .find(|obj| obj.instance_id == scan_target_platform_obj_id)
        .and_then(|obj| obj.property_data.as_point_of_interest_mut())
        .unwrap();
    scan_target_platform_obj.active = 0;

    let actor_blocker_collision_obj = layer.objects.as_mut_vec().iter_mut()
        .find(|obj| obj.instance_id == actor_blocker_collision_id)
        .and_then(|obj| obj.property_data.as_actor_mut())
        .unwrap();
    actor_blocker_collision_obj.active = 0;

    Ok(())
}

fn patch_ore_processing_destructible_rock_pal(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];

    let platform_obj_id = 0x60372;
    let scan_target_platform_obj_id = 0x60378;
    let actor_blocker_collision_id = 0x60379;

    let platform_obj = layer.objects.as_mut_vec().iter_mut()
        .find(|obj| obj.instance_id == platform_obj_id)
        .and_then(|obj| obj.property_data.as_platform_mut())
        .unwrap();
    platform_obj.active = 0;

    let scan_target_platform_obj = layer.objects.as_mut_vec().iter_mut()
        .find(|obj| obj.instance_id == scan_target_platform_obj_id)
        .and_then(|obj| obj.property_data.as_point_of_interest_mut())
        .unwrap();
    scan_target_platform_obj.active = 0;

    let actor_blocker_collision_obj = layer.objects.as_mut_vec().iter_mut()
        .find(|obj| obj.instance_id == actor_blocker_collision_id)
        .and_then(|obj| obj.property_data.as_actor_mut())
        .unwrap();
    actor_blocker_collision_obj.active = 0;

    Ok(())
}

// Removes all cameras and spawn point repositions in the area
// igoring any provided exlcuded script objects.
// Additionally, shortens any specified timers to 0-ish seconds
// When deciding which objects to patch, the most significant
// byte is ignored
fn patch_remove_cutscenes(
    ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea,
    timers_to_zero: Vec<u32>,
    mut skip_ids: Vec<u32>,
    use_timers_instead_of_relay: bool,
)
    -> Result<(), String>
{
    let room_id = area.mlvl_area.mrea;
    let layer_count = area.layer_flags.layer_count as usize;
    let scly = area.mrea().scly_section_mut();

    let mut camera_ids = Vec::<u32>::new();
    let mut spawn_point_ids = Vec::<u32>::new();
    
    let mut elevator_orientation = [0.0, 0.0, 0.0];

    for i in 0..layer_count {
        let layer = &mut scly.layers.as_mut_vec()[i];
        
        for obj in layer.objects.iter() {
            // If this is an elevator cutscene taking the player up, don't skip it //
            // (skipping it can cause sounds to persist in an annoying fashion)    //
            if is_elevator(room_id.to_u32())
            {
                if obj.property_data.is_camera() {
                    let camera = obj.property_data.as_camera().unwrap();
                    let name = camera.name.clone().into_owned().to_owned().to_str().unwrap().to_string().to_lowercase();
                    if name.contains(&"leaving") {
                        skip_ids.push(obj.instance_id & 0x00FFFFFF);
                    }
                }
                if obj.property_data.is_player_actor() {
                    let player_actor = obj.property_data.as_player_actor().unwrap();
                    let name = player_actor.name.clone().into_owned().to_owned().to_str().unwrap().to_string().to_lowercase();
                    if name.contains(&"leaving") {
                        skip_ids.push(obj.instance_id & 0x00FFFFFF);
                    }
                }
            }

            // Get a list of all camera instance ids
            if !skip_ids.contains(&(obj.instance_id & 0x00FFFFFF))
            && obj.property_data.is_camera() {
                camera_ids.push(obj.instance_id & 0x00FFFFFF);
            }

            // Get a list of all spawn point ids
            if !skip_ids.contains(&(obj.instance_id & 0x00FFFFFF)) && obj.property_data.is_spawn_point()
            && (room_id != 0xf7285979 || i == 4) { // don't patch spawn points in shorelines except for ridley
                spawn_point_ids.push(obj.instance_id & 0x00FFFFFF);
            }

            if obj.property_data.is_player_actor() {
                let rotation = obj.property_data.as_player_actor().unwrap().rotation;
                elevator_orientation[0] = rotation[0];
                elevator_orientation[1] = rotation[1];
                elevator_orientation[2] = rotation[2];
            }
        }
    }

    let mut id0 = 0xFFFFFFFF;
    if room_id == 0x0749DF46 || room_id == 0x7A3AD91E {
        id0 = ps.fresh_instance_id_range.next().unwrap();

        let target_object_id = {
            if room_id == 0x0749DF46 { // subchamber 2
                0x0007000B
            } else { // subchamber 3
                0x00080016
            }
        };

        // add a timer to turn activate prime
        scly.layers.as_mut_vec()[0].objects.as_mut_vec().push(structs::SclyObject {
            instance_id: id0,
            property_data: structs::Timer {
                name: b"activate-prime\0".as_cstr(),
                start_time: 1.0,
                max_random_add: 0.0,
                reset_to_zero: 0,
                start_immediately: 0,
                active: 1,
            }.into(),
            connections: vec![
                structs::Connection {
                    state: structs::ConnectionState::ZERO,
                    message: structs::ConnectionMsg::START,
                    target_object_id,
                },
            ].into(),
        });
    }

    // for each layer
    for i in 0..layer_count {
        let layer = &mut scly.layers.as_mut_vec()[i];
        let mut objs_to_add = Vec::<structs::SclyObject>::new();

        // for each object in the layer
        for obj in layer.objects.as_mut_vec() {
            let obj_id = obj.instance_id & 0x00FFFFFF; // remove uper encoding byte

            // If this is an elevator cutscene skip, orient the player towards the door
            if is_elevator(room_id.to_u32()) && obj.property_data.is_spawn_point() {
                obj.property_data.as_spawn_point_mut().unwrap().rotation = elevator_orientation.into();
            }

            // If it's a cutscene-related timer, make it nearly instantaneous
            if obj.property_data.is_timer() {
                let timer = obj.property_data.as_timer_mut().unwrap();

                if timers_to_zero.contains(&obj_id) {
                    if obj_id == 0x0008024E {
                        timer.start_time = 3.0; // chozo ice temple hands
                    } else {
                        timer.start_time = 0.1;
                    }
                }
            }

            // for each connection in that object
            for connection in obj.connections.as_mut_vec().iter_mut() {
                // if this object sends messages to a camera, change the message to be
                // appropriate for a relay
                if camera_ids.contains(&(connection.target_object_id & 0x00FFFFFF)) { 
                    if connection.message == structs::ConnectionMsg::ACTIVATE {
                        connection.message = structs::ConnectionMsg::SET_TO_ZERO;
                    }
                }
            }

            // remove every connection to a spawn point, effectively removing all repositions
            obj.connections.as_mut_vec().retain(|conn| !spawn_point_ids.contains(&(conn.target_object_id & 0x00FFFFFF)));

            // if the object is a camera, create a relay with the same id
            if camera_ids.contains(&obj_id) {
                let mut relay = {
                    structs::SclyObject {
                        instance_id: obj.instance_id,
                        connections: obj.connections.clone(),
                        property_data: structs::SclyProperty::Relay(Box::new(
                            structs::Relay {
                                name: b"camera-relay\0".as_cstr(),
                                active: 1,
                            }
                        ))
                    }
                };

                let shot_duration = {
                    if timers_to_zero.contains(&obj_id) {
                        0.1
                    } else {
                        if room_id == 0x2398E906 {
                            obj.property_data.as_camera().unwrap().shot_duration
                        } else {
                            0.1
                        }
                    }
                };

                let timer_id = ps.fresh_instance_id_range.next().unwrap();
                let mut timer = structs::SclyObject {
                    instance_id: timer_id,
                    property_data: structs::Timer {
                        name: b"cutscene-replacement\0".as_cstr(),
                        start_time: shot_duration,
                        max_random_add: 0.0,
                        reset_to_zero: 0,
                        start_immediately: 0,
                        active: 1,
                    }.into(),
                    connections: vec![].into(),
                };

                let relay_connections = relay.connections.as_mut_vec();
                for connection in relay_connections.iter_mut() {
                    if connection.state == structs::ConnectionState::INACTIVE && use_timers_instead_of_relay {
                        timer.connections.as_mut_vec().push(structs::Connection {
                            state: structs::ConnectionState::ZERO,
                            message: connection.message,
                            target_object_id: connection.target_object_id,
                        });
                    } else if connection.state == structs::ConnectionState::ACTIVE || (connection.state == structs::ConnectionState::INACTIVE && !use_timers_instead_of_relay) {
                        connection.state = structs::ConnectionState::ZERO;
                    }
                }

                if use_timers_instead_of_relay {
                    relay_connections.push(structs::Connection {
                        state: structs::ConnectionState::ZERO,
                        message:  structs::ConnectionMsg::RESET_AND_START,
                        target_object_id: timer_id,
                    });

                    relay_connections.retain(|conn| conn.state != structs::ConnectionState::INACTIVE);
                    objs_to_add.push(timer);
                }

                objs_to_add.push(relay);
            }

            if skip_ids.contains(&obj_id) {
                continue;
            }

            // Special handling for specific rooms //
            if obj_id == 0x00250123 { // flaahgra death cutscene (first camera)
                // teleport the player at end of shot (4.0s), this is long enough for
                // the water to change from acid to water, thus granting pre-floaty
                obj.connections.as_mut_vec().push(structs::Connection {
                    state: structs::ConnectionState::INACTIVE,
                    message: structs::ConnectionMsg::SET_TO_ZERO,
                    target_object_id: 0x04252FC0, // spawn point by item
                });
            } else if obj_id == 0x001E027E { // observatory scan
                // just cut out all the confusion by having the scan always active
                obj.property_data.as_point_of_interest_mut().unwrap().active = 1;
            } else if obj_id == 0x00170153 && !skip_ids.contains(&obj_id) { // magmoor workstation cutscene (power activated)
                // play this cutscene, but only for a second
                // this is to allow players to get floaty jump without having red mist
                obj.property_data.as_camera_mut().unwrap().shot_duration = 3.3;
            } else if obj_id == 0x00070062 { // subchamber 2 trigger
                // When the player enters the room (properly), start the fight
                obj.connections.as_mut_vec().push(structs::Connection {
                    state: structs::ConnectionState::ENTERED,
                    message: structs::ConnectionMsg::RESET_AND_START,
                    target_object_id: id0, // timer
                });
                let trigger = obj.property_data.as_trigger_mut().unwrap();
                trigger.scale[2] = 8.0;
                trigger.position[2] = trigger.position[2] - 11.7;
                trigger.deactivate_on_enter = 1;
            } else if obj_id == 0x00080058 { // subchamber 3 trigger
                // When the player enters the room (properly), start the fight
                obj.connections.as_mut_vec().push(structs::Connection {
                    state: structs::ConnectionState::ENTERED,
                    message: structs::ConnectionMsg::RESET_AND_START,
                    target_object_id: id0, // timer
                });
                let trigger = obj.property_data.as_trigger_mut().unwrap();
                trigger.scale[2] = 8.0;
                trigger.position[2] = trigger.position[2] - 11.7;
                trigger.deactivate_on_enter = 1;
            } else if obj_id == 0x0009005A { // subchamber 4 trigger
                // When the player enters the room (properly), start the fight
                obj.connections.as_mut_vec().push(structs::Connection {
                    state: structs::ConnectionState::INSIDE, // inside, because it's possible to beat exo to this trigger
                    message: structs::ConnectionMsg::START,
                    target_object_id: 0x00090013, // metroid prime
                });
                if obj.property_data.is_trigger() {
                    let trigger = obj.property_data.as_trigger_mut().unwrap();
                    trigger.scale[2] = 5.0;
                    trigger.position[2] = trigger.position[2] - 11.7;
                }
            } else if obj_id == 0x001201AB { // ventillation shaft end timer
                // Disable gas at end of cutscene, not beggining
                obj.connections.as_mut_vec().push(structs::Connection {
                    state: structs::ConnectionState::ZERO,
                    message: structs::ConnectionMsg::DEACTIVATE,
                    target_object_id: 0x001200C2, // gas damage trigger
                });
            } else if vec![0x001200B8, 0x001200B7, 0x001200B6, 0x001200B5, 0x001200B4, 0x001200B2].contains(&obj_id) { // vent shaft puffer
                // increment the dead puffer counter if killed by anything
                obj.connections.as_mut_vec().push(structs::Connection {
                    state: structs::ConnectionState::DEAD,
                    message: structs::ConnectionMsg::INCREMENT,
                    target_object_id: 0x00120094, // dead puffer counter
                });
            } else if obj_id == 0x00120060 { // kill puffer trigger
                // the puffers will increment the counter instead of me, the kill trigger
                obj.connections.as_mut_vec().retain(|_conn| false);
            } else if obj_id == 0x001B065F { // central dynamo collision blocker
                // the power bomb rock collision should not extend beyond the door                
                let actor = obj.property_data.as_actor_mut().unwrap();
                actor.hitbox[1] = 0.4;
                actor.position[1] = actor.position[1] - 0.8;
            } else if obj_id == 0x0002023E { // main plaza turn crane left relay
                // snap the crane immediately so fast players don't fall through the intangible animation
                obj.connections.as_mut_vec().push(structs::Connection {
                    state: structs::ConnectionState::ZERO,
                    message: structs::ConnectionMsg::ACTIVATE,
                    target_object_id: 0x0002001F, // platform
                });

                // set to left relay
                obj.connections.as_mut_vec().push(structs::Connection {
                    state: structs::ConnectionState::ZERO,
                    message: structs::ConnectionMsg::SET_TO_ZERO,
                    target_object_id: 0x0002025B,
                });
            } else if obj_id == 0x00130141 { // arboretum disable gate timer
                // Disable glowey gate marks with gate
                for target_object_id in vec![0x00130119, 0x00130118, 0x0013011F, 0x0013011E] { // glowy symbols
                    obj.connections.as_mut_vec().push(structs::Connection {
                        state: structs::ConnectionState::ZERO,
                        message: structs::ConnectionMsg::DEACTIVATE,
                        target_object_id,
                    });
                }
            }

            // ball triggers can be mean sometimes when not in the saftey of a cutscene, tone it down from 40 to 10
            if obj.property_data.is_ball_trigger() && room_id != 0xEF069019 {
                let ball_trigger = obj.property_data.as_ball_trigger_mut().unwrap();
                ball_trigger.force = 10.0;
            }
        }

        // add all relays
        for obj in objs_to_add.iter() {
            layer.objects.as_mut_vec().push(obj.clone());
        }

        // remove all cutscene related objects from layer
        if room_id == 0xf7285979 && i != 4 // the ridley cutscene is okay
        {
            // special shorelines handling
            let shorelines_triggers = vec![
                0x00020155, // intro cutscene
                0x000201F4, // shorelines tower cutscene
            ];

            layer.objects.as_mut_vec().retain(|obj|
                skip_ids.contains(&(&obj.instance_id & 0x00FFFFFF)) || // except for exluded objects
                !(shorelines_triggers.contains(&(&obj.instance_id & 0x00FFFFFF)))
            );
        }
        else if room_id == 0xb4b41c48 { // keep the cinematic stuff in end cinema
            layer.objects.as_mut_vec().retain(|obj| !obj.property_data.is_camera());
        }
        else
        {
            layer.objects.as_mut_vec().retain(|obj|
                skip_ids.contains(&(&obj.instance_id & 0x00FFFFFF)) || // except for exluded objects
                !(
                    obj.property_data.is_camera() ||
                    obj.property_data.is_camera_filter_keyframe() ||
                    obj.property_data.is_camera_blur_keyframe() ||
                    obj.property_data.is_player_actor() ||
                    vec![0x0018028E, 0x001802A1, 0x0018025C, 0x001800CC, 0x00180212, 0x00020473, 0x00070521].contains(&(obj.instance_id&0x00FFFFFF)) || // thardus death sounds + thardus corpse + main quarry, security station playerhint
                    (obj.property_data.is_special_function() && obj.property_data.as_special_function().unwrap().type_ == 0x18) // "show billboard"
                )
            );
        }        
    }

    Ok(())
}

fn patch_fix_central_dynamo_crash(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
-> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    for layer in scly.layers.as_mut_vec() {
        // find quarantine access door damage trigger
        for obj in layer.objects.as_mut_vec() {
            if obj.instance_id&0x00FFFFFF == 0x001B0470 {
                obj.connections.as_mut_vec().push(structs::Connection {
                    state: structs::ConnectionState::DEAD,
                    message: structs::ConnectionMsg::SET_TO_ZERO,
                    target_object_id: 0x001B03FA, // turn off maze relay
                });
                obj.connections.as_mut_vec().push(structs::Connection {
                    state: structs::ConnectionState::DEAD,
                    message: structs::ConnectionMsg::ACTIVATE,
                    target_object_id: 0x001B02F2, // close the hole
                });
            }
        }
    }

    Ok(())
}

fn patch_main_quarry_door_lock_pal(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[7];

    let locked_door_actor_obj_id = 0x1c0205db;

    let locked_door_actor_obj = layer.objects.as_mut_vec().iter_mut()
        .find(|obj| obj.instance_id == locked_door_actor_obj_id)
        .and_then(|obj| obj.property_data.as_actor_mut())
        .unwrap();
    locked_door_actor_obj.active = 0;

    Ok(())
}

fn patch_mines_security_station_soft_lock<'r>(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    for layer in scly.layers.as_mut_vec().iter_mut() {
        for obj in layer.objects.as_mut_vec().iter_mut() {
            if (obj.instance_id & 0x00FFFFFF) != 0x0007033F {
                continue;
            }
            let trigger = obj.property_data.as_trigger_mut().unwrap();
            trigger.scale[0] = 50.0;
            trigger.scale[1] = 100.0;
            trigger.scale[2] = 40.0;
        }
    }

    Ok(())
}

fn patch_research_core_access_soft_lock(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();

    const DRONE_IDS: &[u32] = &[
                        0x082C006C,
                        0x082C0124,
                    ];
    const RELAY_ENABLE_LOCK_IDS: &[u32] = &[
                        0x082C00CF,
                        0x082C010E,
                    ];
    let trigger_alert_drones_id = 0x082C00CD;

    let trigger_alert_drones_obj = scly.layers.as_mut_vec()[2].objects.iter_mut()
        .find(|i| i.instance_id == trigger_alert_drones_id).unwrap();
    trigger_alert_drones_obj.connections.as_mut_vec().retain(|i| i.target_object_id != RELAY_ENABLE_LOCK_IDS[0] && i.target_object_id != RELAY_ENABLE_LOCK_IDS[1]);

    for drone_id in DRONE_IDS {
        scly.layers.as_mut_vec()[2].objects.iter_mut()
            .find(|i| i.instance_id == *drone_id).unwrap()
            .connections.as_mut_vec().extend_from_slice(
                &[
                    structs::Connection {
                        state: structs::ConnectionState::ZERO,
                        message: structs::ConnectionMsg::SET_TO_ZERO,
                        target_object_id: RELAY_ENABLE_LOCK_IDS[0],
                    },
                    structs::Connection {
                        state: structs::ConnectionState::ZERO,
                        message: structs::ConnectionMsg::SET_TO_ZERO,
                        target_object_id: RELAY_ENABLE_LOCK_IDS[1],
                    },
                ]
            );
    }

    Ok(())
}

fn patch_gravity_chamber_stalactite_grapple_point<'r>(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];

    // Remove the object that turns off the stalactites layer
    layer.objects.as_mut_vec().retain(|obj| obj.instance_id != 3473722);

    Ok(())
}

fn patch_heat_damage_per_sec<'a>(patcher: &mut PrimePatcher<'_, 'a>, heat_damage_per_sec: f32)
{
    const HEATED_ROOMS: &[ResourceInfo] = &[
        resource_info!("06_grapplegallery.MREA"),
        resource_info!("00a_lava_connect.MREA"),
        resource_info!("11_over_muddywaters_b.MREA"),
        resource_info!("00b_lava_connect.MREA"),
        resource_info!("14_over_magdolitepits.MREA"),
        resource_info!("00c_lava_connect.MREA"),
        resource_info!("09_over_monitortower.MREA"),
        resource_info!("00d_lava_connect.MREA"),
        resource_info!("09_lava_pickup.MREA"),
        resource_info!("00e_lava_connect.MREA"),
        resource_info!("12_over_fieryshores.MREA"),
        resource_info!("00f_lava_connect.MREA"),
        resource_info!("00g_lava_connect.MREA"),
    ];

    for heated_room in HEATED_ROOMS.iter() {
        patcher.add_scly_patch((*heated_room).into(), move |_ps, area| {
            let scly = area.mrea().scly_section_mut();
            let layer = &mut scly.layers.as_mut_vec()[0];
            layer.objects.iter_mut()
                .filter_map(|obj| obj.property_data.as_special_function_mut())
                .filter(|sf| sf.type_ == 18) // Is Area Damage function
                .for_each(|sf| sf.unknown1 = heat_damage_per_sec);
            Ok(())
        });
    }
}

fn patch_save_station_for_warp_to_start<'r>(
    ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    game_resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
    spawn_room: SpawnRoomData,
    version: Version,
    warp_to_start_delay_s: f32,
) -> Result<(), String>
{
    let mut warp_to_start_delay_s = warp_to_start_delay_s;
    if warp_to_start_delay_s < 3.0 {
        warp_to_start_delay_s = 3.0
    }

    area.add_dependencies(
        &game_resources,
        0,
        iter::once(custom_asset_ids::WARPING_TO_START_STRG.into())
    );
    area.add_dependencies(
        &game_resources,
        0,
        iter::once(custom_asset_ids::WARPING_TO_START_DELAY_STRG.into())
    );

    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];
    let world_transporter_id = ps.fresh_instance_id_range.next().unwrap();
    let timer_id = ps.fresh_instance_id_range.next().unwrap();
    let hudmemo_id = ps.fresh_instance_id_range.next().unwrap();
    let player_hint_id = ps.fresh_instance_id_range.next().unwrap();

    // Add world transporter leading to starting room
    layer.objects
         .as_mut_vec()
         .push(structs::SclyObject {
            instance_id: world_transporter_id,
            property_data: structs::WorldTransporter::warp(
                spawn_room.mlvl,
                spawn_room.mrea,
                "Warp to Start",
                resource_info!("Deface14B_O.FONT").try_into().unwrap(),
                ResId::new(custom_asset_ids::WARPING_TO_START_STRG.to_u32()),
                version == Version::Pal
            ).into(),
            connections: vec![].into(),
        });

    // Add timer to delay warp (can crash if player warps too quickly)
    layer.objects
         .as_mut_vec()
         .push(structs::SclyObject {
            instance_id: timer_id,
            property_data: structs::Timer {
                name: b"Warp to start delay\0".as_cstr(),

                start_time: warp_to_start_delay_s,
                max_random_add: 0.0,
                reset_to_zero: 0,
                start_immediately: 0,
                active: 1,
            }.into(),
            connections: vec![
                structs::Connection {
                    target_object_id: world_transporter_id,
                    state: structs::ConnectionState::ZERO,
                    message: structs::ConnectionMsg::SET_TO_ZERO,
                },
            ].into(),
        });

    // Inform the player that they are about to be warped
    layer.objects
        .as_mut_vec()
        .push(structs::SclyObject {
           instance_id: hudmemo_id,
           property_data: structs::HudMemo {
                name: b"Warping hudmemo\0".as_cstr(),

                first_message_timer: warp_to_start_delay_s,
                unknown: 1,
                memo_type: 0,
                strg: custom_asset_ids::WARPING_TO_START_DELAY_STRG,
                active: 1,
            }.into(),
           connections: vec![].into(),
       });
    
    // Stop the player from moving
    layer.objects
        .as_mut_vec()
        .push(structs::SclyObject {
           instance_id: player_hint_id,
           property_data: structs::PlayerHint {
            
            name: b"Warping playerhint\0".as_cstr(),

            position: [0.0, 0.0, 0.0].into(),
            rotation: [0.0, 0.0, 0.0].into(),
        
            unknown0: 1, // active
        
            inner_struct: structs::PlayerHintStruct {
                unknowns: [
                    0,
                    0,
                    0,
                    0,
                    0,
                    1, // disable
                    1, // disable
                    1, // disable
                    1, // disable
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                ].into(),
            }.into(),

            unknown1: 10, // priority
           }.into(),
           connections: vec![].into(),
       });

    for obj in layer.objects.iter_mut() {
        if let Some(sp_function) = obj.property_data.as_special_function_mut() {
            if sp_function.type_ == 7 { // Is Save Station function
                obj.connections
                   .as_mut_vec()
                   .push(structs::Connection {
                        target_object_id: timer_id,
                        state: structs::ConnectionState::RETREAT,
                        message: structs::ConnectionMsg::RESET_AND_START,
                    });
                obj.connections
                    .as_mut_vec()
                    .push(structs::Connection {
                         target_object_id: hudmemo_id,
                         state: structs::ConnectionState::RETREAT,
                         message: structs::ConnectionMsg::SET_TO_ZERO,
                     });
                obj.connections
                    .as_mut_vec()
                    .push(structs::Connection {
                        target_object_id: player_hint_id,
                        state: structs::ConnectionState::RETREAT,
                        message: structs::ConnectionMsg::INCREMENT,
                    });
            }
        }
    }

    Ok(())
}

fn patch_memorycard_strg(res: &mut structs::Resource) -> Result<(), String>
{
    let strings = res.kind.as_strg_mut().unwrap()
        .string_tables
        .as_mut_vec()
        .iter_mut()
        .find(|table| table.lang == b"ENGL".into())
        .unwrap()
        .strings
        .as_mut_vec();

    let s = strings.iter_mut()
        .find(|s| *s == "Save progress to Memory Card in Slot A?\u{0}")
        .unwrap();
    *s = "Save progress to Memory Card in Slot A?\nHold &image=SI,0.70,0.68,46434ED3; + &image=SI,0.70,0.68,08A2E4B9; while choosing No to warp to starting area.\u{0}".to_string().into();

    Ok(())
}

fn patch_main_strg(res: &mut structs::Resource, msg: &str) -> Result<(), String>
{
    let strings = res.kind.as_strg_mut().unwrap()
        .string_tables
        .as_mut_vec()
        .iter_mut()
        .find(|table| table.lang == b"ENGL".into())
        .unwrap()
        .strings
        .as_mut_vec();

    let s = strings.iter_mut()
        .find(|s| *s == "Metroid Fusion Connection Bonuses\u{0}")
        .unwrap();
    *s = "Extras\u{0}".to_string().into();

    strings.push(format!("{}\0", msg).into());
    Ok(())
}

fn patch_main_menu(res: &mut structs::Resource) -> Result<(), String>
{
    let frme = res.kind.as_frme_mut().unwrap();

    let (jpn_font, jpn_point_scale) = if frme.version == 0 {
        (None, None)
    } else {
        (Some(ResId::new(0x5d696116)), Some([237, 35].into()))
    };

    frme.widgets.as_mut_vec().push(structs::FrmeWidget {
        name: b"textpane_identifier\0".as_cstr(),
        parent: b"kGSYS_HeadWidgetID\0".as_cstr(),
        use_anim_controller: 0,
        default_visible: 1,
        default_active: 1,
        cull_faces: 0,
        color: [1.0, 1.0, 1.0, 1.0].into(),
        model_draw_flags: 2,
        kind: structs::FrmeWidgetKind::TextPane(
            structs::TextPaneWidget {
                x_dim: 10.455326,
                z_dim: 1.813613,
                scale_center: [
                    -5.227663,
                    0.0,
                    -0.51,
                ].into(),
                font: resource_info!("Deface14B_O.FONT").try_into().unwrap(),
                word_wrap: 0,
                horizontal: 1,
                justification: 0,
                vertical_justification: 0,
                fill_color: [1.0, 1.0, 1.0, 1.0].into(),
                outline_color: [0.0, 0.0, 0.0, 1.0].into(),
                block_extent: [213.0, 38.0].into(),
                jpn_font,
                jpn_point_scale,
            },
        ),
        worker_id: None,
        origin: [9.25, 1.500001, 0.0].into(),
        basis: [
            1.0, 0.0, 0.0,
            0.0, 1.0, 0.0,
            0.0, 0.0, 1.0,
        ].into(),
        rotation_center: [0.0, 0.0, 0.0].into(),
        unknown0: 0,
        unknown1: 0,
    });

    let mut shadow_widget = frme.widgets.as_mut_vec().last().unwrap().clone();
    shadow_widget.name = b"textpane_identifierb\0".as_cstr();
    let tp = match &mut shadow_widget.kind {
        structs::FrmeWidgetKind::TextPane(tp) => tp,
        _ => unreachable!(),
    };
    tp.fill_color = [0.0, 0.0, 0.0, 0.4].into();
    tp.outline_color = [0.0, 0.0, 0.0, 0.2].into();
    shadow_widget.origin[0] -= -0.235091;
    shadow_widget.origin[1] -= -0.104353;
    shadow_widget.origin[2] -= 0.176318;

    frme.widgets.as_mut_vec().push(shadow_widget);

    Ok(())
}

fn patch_credits(
    res: &mut structs::Resource,
    config: &PatchConfig,
)
    -> Result<(), String>
{
    let mut output = "\n\n\n\n\n\n\n".to_string();

    if config.credits_string.is_some() {
        output = format!("{}{}", output, config.credits_string.as_ref().unwrap());
    } else {
        output = format!(
            "{}{}",
            output,
            concat!(
                "&push;&font=C29C51F1;&main-color=#89D6FF;",
                "Major Item Locations",
                "&pop;",
            ).to_owned()
        );

        use std::fmt::Write;
        const PICKUPS_TO_PRINT: &[PickupType] = &[
            PickupType::ScanVisor,
            PickupType::ThermalVisor,
            PickupType::XRayVisor,
            PickupType::VariaSuit,
            PickupType::GravitySuit,
            PickupType::PhazonSuit,
            PickupType::MorphBall,
            PickupType::BoostBall,
            PickupType::SpiderBall,
            PickupType::MorphBallBomb,
            PickupType::PowerBomb,
            PickupType::ChargeBeam,
            PickupType::SpaceJumpBoots,
            PickupType::GrappleBeam,
            PickupType::SuperMissile,
            PickupType::Wavebuster,
            PickupType::IceSpreader,
            PickupType::Flamethrower,
            PickupType::WaveBeam,
            PickupType::IceBeam,
            PickupType::PlasmaBeam
        ];

        for pickup_type in PICKUPS_TO_PRINT {
            let room_name = {
                let mut _room_name = String::new();
                for (_, level) in config.level_data.iter() {
                    for (room_name, room) in level.rooms.iter() {
                        if room.pickups.is_none() { continue };
                        for pickup_info in room.pickups.as_ref().unwrap().iter() {
                            if PickupType::from_str(pickup_type.name()) == PickupType::from_str(&pickup_info.pickup_type) {
                                _room_name = room_name.to_string();
                                break;
                            }
                        }
                    }
                }

                if _room_name.len() == 0 {
                    _room_name = "<Not Present>".to_string();
                }
    
                _room_name
            };
            let pickup_name = pickup_type.name();
            write!(output, "\n\n{}: {}", pickup_name, room_name).unwrap();
        }
    }
    output = format!("{}{}", output, "\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\0");
    res.kind.as_strg_mut().unwrap().string_tables
        .as_mut_vec()
        .iter_mut()
        .find(|table| table.lang == b"ENGL".into())
        .unwrap()
        .strings
        .as_mut_vec()
        .push(output.into());
    Ok(())
}

fn patch_starting_pickups<'r>(
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    starting_items: &StartingItems,
    show_starting_memo: bool,
    game_resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
) -> Result<(), String>
{
    let room_id = area.mlvl_area.internal_id;
    let layer_count = area.mrea().scly_section_mut().layers.as_mut_vec().len() as u32;

    if show_starting_memo {
        // Turn on "Randomizer - Starting Items popup Layer"
        area.layer_flags.flags |= 1 << layer_count;
        area.add_layer(b"Randomizer - Starting Items popup Layer\0".as_cstr());
    }

    let scly = area.mrea().scly_section_mut();

    let mut next_object_id = 0;

    for obj in scly.layers.as_mut_vec()[0].objects.iter_mut() {
        if next_object_id < obj.instance_id {
            next_object_id = obj.instance_id;
        }
    }

    let timer_starting_items_popup_id = (next_object_id + 1) + (layer_count << 26);
    let hud_memo_starting_items_popup_id = (next_object_id + 2) + (layer_count << 26);
    let special_function_starting_items_popup_id = (next_object_id + 3) + (layer_count << 26);

    for layer in scly.layers.iter_mut() {
        for obj in layer.objects.iter_mut() {
            if let Some(spawn_point) = obj.property_data.as_spawn_point_mut() {
                starting_items.update_spawn_point(spawn_point);
            }
        }
    }

    if show_starting_memo {
        scly.layers.as_mut_vec()[layer_count as usize].objects.as_mut_vec().extend_from_slice(
            &[
                structs::SclyObject {
                    instance_id: timer_starting_items_popup_id,
                    property_data: structs::Timer {
                        name: b"Starting Items popup timer\0".as_cstr(),

                        start_time: 0.025,
                        max_random_add: 0f32,
                        reset_to_zero: 0,
                        start_immediately: 1,
                        active: 1,
                    }.into(),
                    connections: vec![
                        structs::Connection {
                            state: structs::ConnectionState::ZERO,
                            message: structs::ConnectionMsg::SET_TO_ZERO,
                            target_object_id: hud_memo_starting_items_popup_id,
                        },
                        structs::Connection {
                            state: structs::ConnectionState::ZERO,
                            message: structs::ConnectionMsg::DECREMENT,
                            target_object_id: special_function_starting_items_popup_id,
                        },
                    ].into(),
                },
                structs::SclyObject {
                    instance_id: hud_memo_starting_items_popup_id,
                    connections: vec![].into(),
                    property_data: structs::HudMemo {
                        name: b"Starting Items popup hudmemo\0".as_cstr(),

                        first_message_timer: 0.5,
                        unknown: 1,
                        memo_type: 1,
                        strg: custom_asset_ids::STARTING_ITEMS_HUDMEMO_STRG,
                        active: 1,
                    }.into(),
                },
                structs::SclyObject {
                    instance_id: special_function_starting_items_popup_id,
                    connections: vec![].into(),
                    property_data: structs::SpecialFunction::layer_change_fn(
                        b"Disable Starting Items popup Layer\0".as_cstr(),
                        room_id,
                        layer_count,
                    ).into(),
                },
            ]
        );

        area.add_dependencies(
            &game_resources,
            0,
            iter::once(custom_asset_ids::STARTING_ITEMS_HUDMEMO_STRG.into())
        );
    }
    Ok(())
}

include!("../compile_to_ppc/patches_config.rs");
fn create_rel_config_file(
    spawn_room: SpawnRoomData,
    quickplay: bool,
) -> Vec<u8>
{
    let config = RelConfig {
        quickplay_mlvl: if quickplay { spawn_room.mlvl } else { 0xFFFFFFFF },
        quickplay_mrea: if quickplay { spawn_room.mrea } else { 0xFFFFFFFF },
    };
    let mut buf = vec![0; mem::size_of::<RelConfig>()];
    ssmarshal::serialize(&mut buf, &config).unwrap();
    buf
}

fn patch_dol<'r>(
    file: &mut structs::FstEntryFile,
    spawn_room: SpawnRoomData,
    version: Version,
    config: &PatchConfig,
    remove_ball_color: bool,
) -> Result<(), String>
{
    if version == Version::NtscJ || version == Version::NtscUTrilogy || version == Version::NtscJTrilogy || version == Version::PalTrilogy {
        return Ok(())
    }

    macro_rules! symbol_addr {
        ($sym:tt, $version:expr) => {
            {
                let s = mp1_symbol!($sym);
                match &$version {
                    Version::NtscU0_00    => s.addr_0_00,
                    Version::NtscU0_01    => unreachable!(),
                    Version::NtscU0_02    => s.addr_0_02,
                    Version::NtscJ    => unreachable!(),
                    Version::Pal         => s.addr_pal,
                    Version::NtscUTrilogy => unreachable!(),
                    Version::NtscJTrilogy => unreachable!(),
                    Version::PalTrilogy => unreachable!(),
                }.unwrap_or_else(|| panic!("Symbol {} unknown for version {}", $sym, $version))
            }
        }
    }

    let reader = match *file {
        structs::FstEntryFile::Unknown(ref reader) => reader.clone(),
        _ => panic!(),
    };

    let mut dol_patcher = DolPatcher::new(reader);
    if version == Version::Pal {
        dol_patcher
            .patch(symbol_addr!("aMetroidprime", version), b"randomprime\0"[..].into())?;
    } else {
        dol_patcher
            .patch(symbol_addr!("aMetroidprimeA", version), b"randomprime A\0"[..].into())?
            .patch(symbol_addr!("aMetroidprimeB", version), b"randomprime B\0"[..].into())?;
    }

    // TODO: missing 1 color (boost ball related)
    if remove_ball_color {
        let ball_color_patch = ppcasm!(symbol_addr!("skBallInnerGlowColors", version), {
            .asciiz b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
        });
        dol_patcher.ppcasm_patch(&ball_color_patch)?;
        let ball_color_patch = ppcasm!(symbol_addr!("BallAuxGlowColors", version), {
            .asciiz b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
        });
        dol_patcher.ppcasm_patch(&ball_color_patch)?;
        let ball_color_patch = ppcasm!(symbol_addr!("BallTransFlashColors", version), {
            .asciiz b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
        });
        dol_patcher.ppcasm_patch(&ball_color_patch)?;
        let ball_color_patch = ppcasm!(symbol_addr!("BallSwooshColors", version), {
            .asciiz b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
        });
        dol_patcher.ppcasm_patch(&ball_color_patch)?;
        let ball_color_patch = ppcasm!(symbol_addr!("BallSwooshColorsJaggy", version), {
            .asciiz b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
        });
        dol_patcher.ppcasm_patch(&ball_color_patch)?;
        let ball_color_patch = ppcasm!(symbol_addr!("BallSwooshColorsCharged", version), {
            .asciiz b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
        });
        dol_patcher.ppcasm_patch(&ball_color_patch)?;
        let ball_color_patch = ppcasm!(symbol_addr!("BallGlowColors", version), {
            .asciiz b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
        });
        dol_patcher.ppcasm_patch(&ball_color_patch)?;
    }

    if config.automatic_crash_screen {
        let automatic_crash_patch = ppcasm!(symbol_addr!("CrashScreenControllerPollBranch", version), {
            nop;
        });
        dol_patcher.ppcasm_patch(&automatic_crash_patch)?;
    }

    let cinematic_skip_patch = ppcasm!(symbol_addr!("ShouldSkipCinematic__22CScriptSpecialFunctionFR13CStateManager", version), {
            li      r3, 0x1;
            blr;
    });
    dol_patcher.ppcasm_patch(&cinematic_skip_patch)?;

    if version == Version::Pal {
        let unlockables_default_ctor_patch = ppcasm!(symbol_addr!("__ct__14CSystemOptionsFv", version) + 0x1dc, {
            li      r6, 100;
            stw     r6, 0x80(r31);
            lis     r6, 0xF7FF;
            stw     r6, 0x84(r31);
        });
        dol_patcher.ppcasm_patch(&unlockables_default_ctor_patch)?;
    } else {
        let unlockables_default_ctor_patch = ppcasm!(symbol_addr!("__ct__14CSystemOptionsFv", version) + 0x194, {
            li      r6, 100;
            stw     r6, 0xcc(r3);
            lis     r6, 0xF7FF;
            stw     r6, 0xd0(r3);
        });
        dol_patcher.ppcasm_patch(&unlockables_default_ctor_patch)?;
    };

    if version == Version::Pal {
        let unlockables_read_ctor_patch = ppcasm!(symbol_addr!("__ct__14CSystemOptionsFRC12CInputStream", version) + 0x330, {
            li      r6, 100;
            stw     r6, 0x80(r28);
            lis     r6, 0xF7FF;
            stw     r6, 0x84(r28);
            mr      r3, r29;
            li      r4, 2;
        });
        dol_patcher.ppcasm_patch(&unlockables_read_ctor_patch)?;
    } else {
        let unlockables_read_ctor_patch = ppcasm!(symbol_addr!("__ct__14CSystemOptionsFRC12CInputStream", version) + 0x308, {
            li      r6, 100;
            stw     r6, 0xcc(r28);
            lis     r6, 0xF7FF;
            stw     r6, 0xd0(r28);
            mr      r3, r29;
            li      r4, 2;
        });
        dol_patcher.ppcasm_patch(&unlockables_read_ctor_patch)?;
    };

    if version != Version::Pal {
        let missile_hud_formating_patch = ppcasm!(symbol_addr!("SetNumMissiles__20CHudMissileInterfaceFiRC13CStateManager", version) + 0x14, {
                b          skip;
            fmt:
                .asciiz b"%03d/%03d";

            skip:
                stw        r30, 40(r1);// var_8(r1);
                mr         r30, r3;
                stw        r4, 8(r1);// var_28(r1)

                lwz        r6, 4(r30);

                mr         r5, r4;

                lis        r4, fmt@h;
                addi       r4, r4, fmt@l;

                addi       r3, r1, 12;// arg_C

                nop; // crclr      cr6;
                bl         { symbol_addr!("sprintf", version) };

                addi       r3, r1, 20;// arg_14;
                addi       r4, r1, 12;// arg_C
        });
        dol_patcher.ppcasm_patch(&missile_hud_formating_patch)?;
    }

    if config.qol_cosmetic {
        let powerbomb_hud_formating_patch = ppcasm!(symbol_addr!("SetBombParams__17CHudBallInterfaceFiiibbb", version) + 0x2c, {
                b skip;
            fmt:
                .asciiz b"%d/%d";// %d";
                nop;
            skip:
                mr         r6, r27;
                mr         r5, r28;
                lis        r4, fmt@h;
                addi       r4, r4, fmt@l;
                addi       r3, r1, 12;// arg_C;
                nop; // crclr      cr6;
                bl         { symbol_addr!("sprintf", version) };

        });
        dol_patcher.ppcasm_patch(&powerbomb_hud_formating_patch)?;
    }

    if version == Version::Pal {
        let level_select_mlvl_upper_patch = ppcasm!(symbol_addr!("__sinit_CFrontEndUI_cpp", version) + 0x0c, {
                lis         r3, {spawn_room.mlvl}@h;
        });
        dol_patcher.ppcasm_patch(&level_select_mlvl_upper_patch)?;

        let level_select_mlvl_lower_patch = ppcasm!(symbol_addr!("__sinit_CFrontEndUI_cpp", version) + 0x18, {
                addi        r0, r3, {spawn_room.mlvl}@l;
        });
        dol_patcher.ppcasm_patch(&level_select_mlvl_lower_patch)?;
    } else {
        let level_select_mlvl_upper_patch = ppcasm!(symbol_addr!("__sinit_CFrontEndUI_cpp", version) + 0x04, {
                lis         r4, {spawn_room.mlvl}@h;
        });
        dol_patcher.ppcasm_patch(&level_select_mlvl_upper_patch)?;

        let level_select_mlvl_lower_patch = ppcasm!(symbol_addr!("__sinit_CFrontEndUI_cpp", version) + 0x10, {
                addi        r0, r4, {spawn_room.mlvl}@l;
        });
        dol_patcher.ppcasm_patch(&level_select_mlvl_lower_patch)?;
    }

    let level_select_mrea_idx_patch = ppcasm!(symbol_addr!("__ct__11CWorldStateFUi", version) + 0x10, {
            li          r0, { spawn_room.mrea_idx };
    });
    dol_patcher.ppcasm_patch(&level_select_mrea_idx_patch)?;

    let disable_hints_setting_patch = ppcasm!(symbol_addr!("ResetToDefaults__12CGameOptionsFv", version) + 0x80, {
            rlwimi      r0, r6, 3, 28, 28;
    });
    dol_patcher.ppcasm_patch(&disable_hints_setting_patch)?;

    if config.nonvaria_heat_damage {
        let heat_damage_patch = ppcasm!(symbol_addr!("ThinkAreaDamage__22CScriptSpecialFunctionFfR13CStateManager", version) + 0x4c, {
                lwz     r4, 0xdc(r4);
                nop;
                subf    r0, r6, r5;
                cntlzw  r0, r0;
                nop;
        });
        dol_patcher.ppcasm_patch(&heat_damage_patch)?;
    }

    if config.staggered_suit_damage {
        let (patch_offset, jump_offset) = if version == Version::Pal {
            (0x11c, 0x1b8)
        } else {
            (0x128, 0x1c4)
        };

        let staggered_suit_damage_patch = ppcasm!(symbol_addr!("ApplyLocalDamage__13CStateManagerFRC9CVector3fRC9CVector3fR6CActorfRC11CWeaponMode", version) + patch_offset, {
                lwz     r3, 0x8b8(r25);
                lwz     r3, 0(r3);
                lwz     r4, 220(r3);
                lwz     r5, 212(r3);
                addc    r4, r4, r5;
                lwz     r5, 228(r3);
                addc    r4, r4, r5;
                rlwinm  r4, r4, 2, 0, 29;
                lis     r6, data@h;
                addi    r6, r6, data@l;
                lfsx     f0, r4, r6;
                b       { symbol_addr!("ApplyLocalDamage__13CStateManagerFRC9CVector3fRC9CVector3fR6CActorfRC11CWeaponMode", version) + jump_offset };
            data:
                .float 0.0;
                .float 0.1;
                .float 0.2;
                .float 0.5;
        });
        dol_patcher.ppcasm_patch(&staggered_suit_damage_patch)?;
    }

    for (pickup_type, value) in &config.item_max_capacity {
        let capacity_patch = ppcasm!(symbol_addr!("CPlayerState_PowerUpMaxValues", version) + pickup_type.kind() * 4, {
            .long *value;
        });
        dol_patcher.ppcasm_patch(&capacity_patch)?;
    }

    // set etank capacity and base health
    let etank_capacity = config.etank_capacity as f32;
    let base_health = etank_capacity - 1.0;
    let etank_capacity_base_health_patch = ppcasm!(symbol_addr!("g_EtankCapacity", version), {
        .float etank_capacity;
        .float base_health;
    });
    dol_patcher.ppcasm_patch(&etank_capacity_base_health_patch)?;

    if version == Version::NtscU0_02 || version == Version::Pal {
        let players_choice_scan_dash_patch = ppcasm!(symbol_addr!("SidewaysDashAllowed__7CPlayerCFffRC11CFinalInputR13CStateManager", version) + 0x3c, {
                b       { symbol_addr!("SidewaysDashAllowed__7CPlayerCFffRC11CFinalInputR13CStateManager", version) + 0x54 };
        });
        dol_patcher.ppcasm_patch(&players_choice_scan_dash_patch)?;
    }

    if config.map_default_state != MapState::Default {
        let is_mapped_patch = ppcasm!(symbol_addr!("IsMapped__13CMapWorldInfoCF7TAreaId", version), {
            li      r3, 0x1;
            blr;
        });
        dol_patcher.ppcasm_patch(&is_mapped_patch)?;
        if config.map_default_state == MapState::Visited {
            let is_area_visited_patch = ppcasm!(symbol_addr!("IsAreaVisited__13CMapWorldInfoCF7TAreaId", version), {
                li      r3, 0x1;
                blr;
            });
            dol_patcher.ppcasm_patch(&is_area_visited_patch)?;
        }
    }

    // Multiworld focused patches
    if config.multiworld_dol_patches {
        // IncrPickUp's switch array for UnknownItem1 to actually give stuff
        let incr_pickup_switch_patch = ppcasm!(symbol_addr!("IncrPickUpSwitchCaseData", version) + 21 * 4, {
            .long symbol_addr!("IncrPickUp__12CPlayerStateFQ212CPlayerState9EItemTypei", version) + 25 * 4;
        });
        dol_patcher.ppcasm_patch(&incr_pickup_switch_patch)?;

        // Remove DecrPickUp checks for the correct item types
        let decr_pickup_patch = ppcasm!(symbol_addr!("DecrPickUp__12CPlayerStateFQ212CPlayerState9EItemTypei", version) + 5 * 4, {
            nop;
            nop;
            nop;
            nop;
            nop;
            nop;
            nop;
        });
        dol_patcher.ppcasm_patch(&decr_pickup_patch)?;
    }

    if let Some(update_hint_state_replacement) = &config.update_hint_state_replacement {
        dol_patcher.patch(symbol_addr!("UpdateHintState__13CStateManagerFf", version), Cow::from(update_hint_state_replacement.clone()))?;
    }
    
    if config.warp_to_start
    {
        let handle_no_to_save_msg_patch = ppcasm!(symbol_addr!("ThinkSaveStation__22CScriptSpecialFunctionFfR13CStateManager", version) + 0x54, {
                b         0x80002200;
        });
        dol_patcher.ppcasm_patch(&handle_no_to_save_msg_patch)?;

        let warp_to_start_patch = ppcasm!(0x80002200, {
                lis       r14, {symbol_addr!("g_Main", version)}@h;
                addi      r14, r14, {symbol_addr!("g_Main", version)}@l;
                lwz       r14, 0x0(r14);
                lwz       r14, 0x164(r14);
                lwz       r14, 0x34(r14);
                lbz       r0, 0x86(r14);
                cmpwi     r0, 0;
                beq       0x80002234;
                lbz       r0, 0x89(r14);
                cmpwi     r0, 0;
                beq       0x80002234;
                li        r4, 12;
                b         0x80002238;
                li        r4, 9;
                andi      r14, r14, 0;
                b         { symbol_addr!("ThinkSaveStation__22CScriptSpecialFunctionFfR13CStateManager", version) + 0x58 };
        });
        dol_patcher.add_text_segment(0x80002200, Cow::Owned(warp_to_start_patch.encoded_bytes()))?;
    }

    // Add rel loader to the binary
    let (rel_loader_bytes, rel_loader_map_str) = match version {
        Version::NtscU0_00 => {
            let loader_bytes = rel_files::REL_LOADER_100;
            let map_str = rel_files::REL_LOADER_100_MAP;
            (loader_bytes, map_str)
        },
        Version::NtscU0_01 => unreachable!(),
        Version::NtscU0_02 => {
            let loader_bytes = rel_files::REL_LOADER_102;
            let map_str = rel_files::REL_LOADER_102_MAP;
            (loader_bytes, map_str)
        },
        Version::NtscJ => unreachable!(),
        Version::Pal => {
            let loader_bytes = rel_files::REL_LOADER_PAL;
            let map_str = rel_files::REL_LOADER_PAL_MAP;
            (loader_bytes, map_str)
        },
        Version::NtscUTrilogy => unreachable!(),
        Version::NtscJTrilogy => unreachable!(),
        Version::PalTrilogy => unreachable!(),
    };

    let mut rel_loader = rel_loader_bytes.to_vec();

    let rel_loader_map = dol_linker::parse_symbol_table(
        "extra_assets/rel_loader_1.0?.bin.map".as_ref(),
        rel_loader_map_str.lines().map(|l| Ok(l.to_owned())),
    ).map_err(|e| e.to_string())?;


    let bytes_needed = ((rel_loader.len() + 31) & !31) - rel_loader.len();
    rel_loader.extend([0; 32][..bytes_needed].iter().copied());

    dol_patcher.add_text_segment(0x80002000, Cow::Owned(rel_loader))?;

    dol_patcher.ppcasm_patch(&ppcasm!(symbol_addr!("PPCSetFpIEEEMode", version) + 4, {
        b      { rel_loader_map["rel_loader_hook"] };
    }))?;


    *file = structs::FstEntryFile::ExternalFile(Box::new(dol_patcher));
    Ok(())
}

fn empty_frigate_pak<'r>(file: &mut structs::FstEntryFile)
    -> Result<(), String>
{
    // To reduce the amount of data that needs to be copied, empty the contents of the pak
    let pak = match file {
        structs::FstEntryFile::Pak(pak) => pak,
        _ => unreachable!(),
    };

    // XXX This is a workaround for a bug in some versions of Nintendont.
    //     The details can be found in a comment on issue #5.
    let res = crate::custom_assets::build_resource_raw(
        0,
        structs::ResourceKind::External(vec![0; 64], b"XXXX".into())
    );
    pak.resources = iter::once(res).collect();
    Ok(())
}

fn patch_ctwk_game(res: &mut structs::Resource, ctwk_config: &CtwkConfig)
    -> Result<(), String>
{
    let mut ctwk = res.kind.as_ctwk_mut().unwrap();
    let mut ctwk_game = match &mut ctwk {
        structs::Ctwk::CtwkGame(i) => i,
        _ => panic!("Failed to map res=0x{:X} as CtwkGame", res.file_id),
    };

    // println!("before - {:?}", ctwk_game);
    ctwk_game.press_start_delay = 0.001;

    if ctwk_config.fov.is_some() {
        ctwk_game.fov = ctwk_config.fov.unwrap();
    }

    if ctwk_config.hardmode_damage_mult.is_some() {
        ctwk_game.hardmode_damage_mult = ctwk_config.hardmode_damage_mult.unwrap();
    }

    if ctwk_config.hardmode_weapon_mult.is_some() {
        ctwk_game.hardmode_weapon_mult = ctwk_config.hardmode_weapon_mult.unwrap();
    }

    if ctwk_config.underwater_fog_distance.is_some() {
        let underwater_fog_distance = ctwk_config.underwater_fog_distance.unwrap();
        ctwk_game.water_fog_distance_base = ctwk_game.water_fog_distance_base*underwater_fog_distance;
        ctwk_game.water_fog_distance_range = ctwk_game.water_fog_distance_range*underwater_fog_distance;
        ctwk_game.gravity_water_fog_distance_base = ctwk_game.gravity_water_fog_distance_base*underwater_fog_distance;
        ctwk_game.gravity_water_fog_distance_range = ctwk_game.gravity_water_fog_distance_range*underwater_fog_distance;
    }

    Ok(())
}

fn patch_ctwk_player(res: &mut structs::Resource, ctwk_config: &CtwkConfig)
-> Result<(), String>
{
    let mut ctwk = res.kind.as_ctwk_mut().unwrap();
    let mut ctwk_player = match &mut ctwk {
        structs::Ctwk::CtwkPlayer(i) => i,
        _ => panic!("Failed to map res=0x{:X} as CtwkPlayer", res.file_id),
    };

    if ctwk_config.player_size.is_some() {
        let player_size = ctwk_config.player_size.unwrap();
        ctwk_player.player_height = ctwk_player.player_height*player_size;
        ctwk_player.player_xy_half_extent = ctwk_player.player_xy_half_extent*player_size;
        ctwk_player.step_up_height = ctwk_player.step_up_height*player_size;
        ctwk_player.step_down_height = ctwk_player.step_down_height*player_size;
    }

    if ctwk_config.step_up_height.is_some() {
        ctwk_player.step_up_height = ctwk_player.step_up_height*ctwk_config.step_up_height.unwrap();
    }

    if ctwk_config.morph_ball_size.is_some() {
        ctwk_player.player_ball_half_extent = ctwk_player.player_ball_half_extent*ctwk_config.morph_ball_size.unwrap();
    }

    if ctwk_config.easy_lava_escape.unwrap_or(false) {
        ctwk_player.lava_jump_factor = 100.0;
        ctwk_player.lava_ball_jump_factor = 100.0;
    }

    // println!("before - {:?}", ctwk_player);
    if ctwk_config.move_while_scan.unwrap_or(false) {
        ctwk_player.scan_freezes_game = 0;
    }

    if ctwk_config.scan_range.is_some() {
        let scan_range = ctwk_config.scan_range.unwrap();

        ctwk_player.scanning_range = scan_range;

        if scan_range > ctwk_player.scan_max_lock_distance {
            ctwk_player.scan_max_lock_distance = scan_range;
        }

        if scan_range > ctwk_player.scan_max_target_distance {
            ctwk_player.scan_max_target_distance = scan_range;
        }        
    }

    if ctwk_config.bomb_jump_height.is_some() {
        ctwk_player.bomb_jump_height = ctwk_player.bomb_jump_height*ctwk_config.bomb_jump_height.unwrap();
    }

    if ctwk_config.bomb_jump_radius.is_some() {
        ctwk_player.bomb_jump_radius = ctwk_player.bomb_jump_radius*ctwk_config.bomb_jump_radius.unwrap();
    }

    if ctwk_config.grapple_beam_speed.is_some() {
        ctwk_player.grapple_beam_speed = ctwk_player.grapple_beam_speed*ctwk_config.grapple_beam_speed.unwrap();
    }

    if ctwk_config.aim_assist_angle.is_some() {
        let aim_assist_angle = ctwk_config.aim_assist_angle.unwrap();
        ctwk_player.aim_assist_vertical_angle = aim_assist_angle;
        ctwk_player.aim_assist_horizontal_angle = aim_assist_angle;
    }

    if ctwk_config.gravity.is_some() {
        ctwk_player.normal_grav_accel = ctwk_player.normal_grav_accel*ctwk_config.gravity.unwrap();
    }

    if ctwk_config.ice_break_timeout.is_some() {
        ctwk_player.frozen_timeout = ctwk_config.ice_break_timeout.unwrap();
    }

    if ctwk_config.ice_break_jump_count.is_some() {
        ctwk_player.ice_break_jump_count = ctwk_config.ice_break_jump_count.unwrap();
    }

    if ctwk_config.ice_break_jump_count.is_some() {
        ctwk_player.ice_break_jump_count = ctwk_config.ice_break_jump_count.unwrap();
    }

    if ctwk_config.ground_friction.is_some() {
        ctwk_player.translation_friction[0] = ctwk_player.translation_friction[0]*ctwk_config.ground_friction.unwrap();
    }

    if ctwk_config.coyote_frames.is_some() {
        ctwk_player.allowed_ledge_time = (ctwk_config.coyote_frames.unwrap() as f32)*(1.0/60.0);
    }

    if ctwk_config.move_during_free_look.unwrap_or(false) {
        ctwk_player.move_during_free_look = 1;
    }

    if ctwk_config.recenter_after_freelook.unwrap_or(false) {
        ctwk_player.freelook_turns_player = 0;
    }
    
    if ctwk_config.toggle_free_look.unwrap_or(false) {
        ctwk_player.hold_buttons_for_free_look = 0;
    }

    if ctwk_config.two_buttons_for_free_look.unwrap_or(false) {
        ctwk_player.two_buttons_for_free_look = 1;
    }

    if ctwk_config.disable_dash.unwrap_or(false) {
        ctwk_player.dash_enabled = 0;
    }
    
    if ctwk_config.varia_damage_reduction.is_some() {
        ctwk_player.varia_damage_reduction = ctwk_player.varia_damage_reduction*ctwk_config.varia_damage_reduction.unwrap();
    }
    
    if ctwk_config.gravity_damage_reduction.is_some() {
        ctwk_player.gravity_damage_reduction = ctwk_player.gravity_damage_reduction*ctwk_config.gravity_damage_reduction.unwrap();
    }

    if ctwk_config.phazon_damage_reduction.is_some() {
        ctwk_player.phazon_damage_reduction = ctwk_player.phazon_damage_reduction*ctwk_config.phazon_damage_reduction.unwrap();
    }

    if ctwk_config.max_speed.is_some() {
        let max_speed = ctwk_config.max_speed.unwrap();
        ctwk_player.translation_max_speed[0] = ctwk_player.translation_max_speed[0]*max_speed;
        ctwk_player.translation_max_speed[1] = ctwk_player.translation_max_speed[1]*max_speed;
        ctwk_player.translation_max_speed[2] = ctwk_player.translation_max_speed[2]*max_speed;
        ctwk_player.translation_max_speed[3] = ctwk_player.translation_max_speed[3]*max_speed;
        ctwk_player.translation_max_speed[4] = ctwk_player.translation_max_speed[4]*max_speed;
        ctwk_player.translation_max_speed[5] = ctwk_player.translation_max_speed[5]*max_speed;
        ctwk_player.translation_max_speed[6] = ctwk_player.translation_max_speed[6]*max_speed;
        ctwk_player.translation_max_speed[7] = ctwk_player.translation_max_speed[7]*max_speed;
    }

    if ctwk_config.max_acceleration.is_some() {
        let max_acceleration = ctwk_config.max_acceleration.unwrap();
        ctwk_player.translation_max_speed[0] = ctwk_player.translation_max_speed[0]*max_acceleration;
        ctwk_player.translation_max_speed[1] = ctwk_player.translation_max_speed[1]*max_acceleration;
        ctwk_player.translation_max_speed[2] = ctwk_player.translation_max_speed[2]*max_acceleration;
        ctwk_player.translation_max_speed[3] = ctwk_player.translation_max_speed[3]*max_acceleration;
        ctwk_player.translation_max_speed[4] = ctwk_player.translation_max_speed[4]*max_acceleration;
        ctwk_player.translation_max_speed[5] = ctwk_player.translation_max_speed[5]*max_acceleration;
        ctwk_player.translation_max_speed[6] = ctwk_player.translation_max_speed[6]*max_acceleration;
        ctwk_player.translation_max_speed[7] = ctwk_player.translation_max_speed[7]*max_acceleration;
    }

    if ctwk_config.space_jump_impulse.is_some() {
        ctwk_player.double_jump_impulse = ctwk_player.double_jump_impulse*ctwk_config.space_jump_impulse.unwrap();
    }
    if ctwk_config.vertical_space_jump_accel.is_some() {
        ctwk_player.vertical_double_jump_accel = ctwk_player.vertical_double_jump_accel*ctwk_config.vertical_space_jump_accel.unwrap();
    }
    if ctwk_config.horizontal_space_jump_accel.is_some() {
        ctwk_player.horizontal_double_jump_accel = ctwk_player.horizontal_double_jump_accel*ctwk_config.horizontal_space_jump_accel.unwrap();
    }

    if ctwk_config.allowed_jump_time.is_some() {
        ctwk_player.allowed_jump_time = ctwk_player.allowed_jump_time*ctwk_config.allowed_jump_time.unwrap();
    }
    if ctwk_config.allowed_space_jump_time.is_some() {
        ctwk_player.allowed_double_jump_time = ctwk_player.allowed_double_jump_time*ctwk_config.allowed_space_jump_time.unwrap();
    }
    if ctwk_config.min_space_jump_window.is_some() {
        ctwk_player.min_double_jump_window = ctwk_player.min_double_jump_window*ctwk_config.min_space_jump_window.unwrap();
    }
    if ctwk_config.max_space_jump_window.is_some() {
        ctwk_player.max_double_jump_window = ctwk_player.max_double_jump_window*ctwk_config.max_space_jump_window.unwrap();
    }
    if ctwk_config.min_jump_time.is_some() {
        ctwk_player.min_jump_time = ctwk_player.min_jump_time*ctwk_config.min_jump_time.unwrap();
    }
    if ctwk_config.min_space_jump_time.is_some() {
        ctwk_player.min_double_jump_time = ctwk_player.min_double_jump_time*ctwk_config.min_space_jump_time.unwrap();
    }
    if ctwk_config.falling_space_jump.is_some() {
        ctwk_player.falling_double_jump = {
            if ctwk_config.falling_space_jump.unwrap() {
                1
            } else {
                0
            }
        };
    }
    if ctwk_config.impulse_space_jump.is_some() {
        ctwk_player.impulse_double_jump = {
            if ctwk_config.impulse_space_jump.unwrap() {
                1
            } else {
                0
            }
        };
    }

    if ctwk_config.eye_offset.is_some() {
        ctwk_player.eye_offset = ctwk_player.eye_offset*ctwk_config.eye_offset.unwrap();
    }

    if ctwk_config.turn_speed.is_some() {
        let turn_speed = ctwk_config.turn_speed.unwrap();
        ctwk_player.turn_speed_multiplier = ctwk_player.turn_speed_multiplier*turn_speed;
        ctwk_player.free_look_turn_speed_multiplier = ctwk_player.free_look_turn_speed_multiplier*turn_speed;
        // there might be others
    }

    Ok(())
}

fn patch_ctwk_player_gun(res: &mut structs::Resource, ctwk_config: &CtwkConfig)
-> Result<(), String>
{
    let mut ctwk = res.kind.as_ctwk_mut().unwrap();
    let ctwk_player_gun = match &mut ctwk {
        structs::Ctwk::CtwkPlayerGun(i) => i,
        _ => panic!("Failed to map res=0x{:X} as CtwkPlayerGun", res.file_id),
    };

    if ctwk_config.gun_position.is_some() {
        let gun_position = ctwk_config.gun_position.unwrap();
        ctwk_player_gun.gun_position[0] = ctwk_player_gun.gun_position[0] + gun_position[0];
        ctwk_player_gun.gun_position[1] = ctwk_player_gun.gun_position[1] + gun_position[1];
        ctwk_player_gun.gun_position[2] = ctwk_player_gun.gun_position[2] + gun_position[2];
    }

    // ctwk_player_gun.beams[0].normal.damage = 9999999.0;

    ctwk_player_gun.beams[0].normal.knockback = ctwk_player_gun.beams[0].normal.knockback*10.0;
    ctwk_player_gun.beams[0].cool_down = 0.00001;
    ctwk_player_gun.beams[1].cool_down = 0.00001;
    ctwk_player_gun.beams[2].cool_down = 0.00001;
    ctwk_player_gun.beams[3].cool_down = 0.00001;
    ctwk_player_gun.beams[4].cool_down = 0.00001;

    ctwk_player_gun.combos[0].radius = 99999.0;
    ctwk_player_gun.combos[0].radius_damage = 99999.0;
    ctwk_player_gun.combos[0].knockback = 99999.0;
    
    ctwk_player_gun.combos[1].radius = 99999.0;
    ctwk_player_gun.combos[1].radius_damage = 99999.0;
    ctwk_player_gun.combos[1].knockback = 99999.0;

    ctwk_player_gun.combos[2].radius = 99999.0;
    ctwk_player_gun.combos[2].radius_damage = 99999.0;
    ctwk_player_gun.combos[2].knockback = 99999.0;

    ctwk_player_gun.combos[3].radius = 99999.0;
    ctwk_player_gun.combos[3].radius_damage = 99999.0;
    ctwk_player_gun.combos[3].knockback = 99999.0;

    ctwk_player_gun.combos[4].radius = 99999.0;
    ctwk_player_gun.combos[4].radius_damage = 99999.0;
    ctwk_player_gun.combos[4].knockback = 99999.0;

    ctwk_player_gun.power_bomb.damage = 9999999.0;
    ctwk_player_gun.power_bomb.radius = 9999999.0;
    ctwk_player_gun.power_bomb.radius_damage = 9999999.0;

    
    Ok(())
}

fn patch_ctwk_ball(res: &mut structs::Resource, ctwk_config: &CtwkConfig)
-> Result<(), String>
{
    let mut ctwk = res.kind.as_ctwk_mut().unwrap();
 
    let ctwk_ball = match &mut ctwk {
        structs::Ctwk::CtwkBall(i) => i,
        _ => panic!("Failed to map res=0x{:X} as CtwkBall", res.file_id),
    };

    if ctwk_config.max_translation_accel.is_some() {
        ctwk_ball.max_translation_accel[0] = ctwk_ball.max_translation_accel[0]*ctwk_config.max_translation_accel.unwrap();
        ctwk_ball.max_translation_accel[1] = ctwk_ball.max_translation_accel[1]*ctwk_config.max_translation_accel.unwrap();
        ctwk_ball.max_translation_accel[2] = ctwk_ball.max_translation_accel[2]*ctwk_config.max_translation_accel.unwrap();
        ctwk_ball.max_translation_accel[3] = ctwk_ball.max_translation_accel[3]*ctwk_config.max_translation_accel.unwrap();
        ctwk_ball.max_translation_accel[4] = ctwk_ball.max_translation_accel[4]*ctwk_config.max_translation_accel.unwrap();
        ctwk_ball.max_translation_accel[5] = ctwk_ball.max_translation_accel[5]*ctwk_config.max_translation_accel.unwrap();
        ctwk_ball.max_translation_accel[6] = ctwk_ball.max_translation_accel[6]*ctwk_config.max_translation_accel.unwrap();
        ctwk_ball.max_translation_accel[7] = ctwk_ball.max_translation_accel[7]*ctwk_config.max_translation_accel.unwrap();
    }
    if ctwk_config.translation_friction.is_some() {
        ctwk_ball.translation_friction[0] = ctwk_ball.translation_friction[0]*ctwk_config.translation_friction.unwrap();
        ctwk_ball.translation_friction[1] = ctwk_ball.translation_friction[1]*ctwk_config.translation_friction.unwrap();
        ctwk_ball.translation_friction[2] = ctwk_ball.translation_friction[2]*ctwk_config.translation_friction.unwrap();
        ctwk_ball.translation_friction[3] = ctwk_ball.translation_friction[3]*ctwk_config.translation_friction.unwrap();
        ctwk_ball.translation_friction[4] = ctwk_ball.translation_friction[4]*ctwk_config.translation_friction.unwrap();
        ctwk_ball.translation_friction[5] = ctwk_ball.translation_friction[5]*ctwk_config.translation_friction.unwrap();
        ctwk_ball.translation_friction[6] = ctwk_ball.translation_friction[6]*ctwk_config.translation_friction.unwrap();
        ctwk_ball.translation_friction[7] = ctwk_ball.translation_friction[7]*ctwk_config.translation_friction.unwrap();
    }
    if ctwk_config.translation_max_speed.is_some() {
        ctwk_ball.translation_max_speed[0] = ctwk_ball.translation_max_speed[0]*ctwk_config.translation_max_speed.unwrap();
        ctwk_ball.translation_max_speed[1] = ctwk_ball.translation_max_speed[1]*ctwk_config.translation_max_speed.unwrap();
        ctwk_ball.translation_max_speed[2] = ctwk_ball.translation_max_speed[2]*ctwk_config.translation_max_speed.unwrap();
        ctwk_ball.translation_max_speed[3] = ctwk_ball.translation_max_speed[3]*ctwk_config.translation_max_speed.unwrap();
        ctwk_ball.translation_max_speed[4] = ctwk_ball.translation_max_speed[4]*ctwk_config.translation_max_speed.unwrap();
        ctwk_ball.translation_max_speed[5] = ctwk_ball.translation_max_speed[5]*ctwk_config.translation_max_speed.unwrap();
        ctwk_ball.translation_max_speed[6] = ctwk_ball.translation_max_speed[6]*ctwk_config.translation_max_speed.unwrap();
        ctwk_ball.translation_max_speed[7] = ctwk_ball.translation_max_speed[7]*ctwk_config.translation_max_speed.unwrap();
    }
    if ctwk_config.ball_forward_braking_accel.is_some() {
        ctwk_ball.ball_forward_braking_accel[0] = ctwk_ball.ball_forward_braking_accel[0]*ctwk_config.ball_forward_braking_accel.unwrap();
        ctwk_ball.ball_forward_braking_accel[1] = ctwk_ball.ball_forward_braking_accel[1]*ctwk_config.ball_forward_braking_accel.unwrap();
        ctwk_ball.ball_forward_braking_accel[2] = ctwk_ball.ball_forward_braking_accel[2]*ctwk_config.ball_forward_braking_accel.unwrap();
        ctwk_ball.ball_forward_braking_accel[3] = ctwk_ball.ball_forward_braking_accel[3]*ctwk_config.ball_forward_braking_accel.unwrap();
        ctwk_ball.ball_forward_braking_accel[4] = ctwk_ball.ball_forward_braking_accel[4]*ctwk_config.ball_forward_braking_accel.unwrap();
        ctwk_ball.ball_forward_braking_accel[5] = ctwk_ball.ball_forward_braking_accel[5]*ctwk_config.ball_forward_braking_accel.unwrap();
        ctwk_ball.ball_forward_braking_accel[6] = ctwk_ball.ball_forward_braking_accel[6]*ctwk_config.ball_forward_braking_accel.unwrap();
        ctwk_ball.ball_forward_braking_accel[7] = ctwk_ball.ball_forward_braking_accel[7]*ctwk_config.ball_forward_braking_accel.unwrap();
    }
    if ctwk_config.ball_gravity.is_some() {
        ctwk_ball.ball_gravity = ctwk_ball.ball_gravity*ctwk_config.ball_gravity.unwrap();
    }
    if ctwk_config.ball_water_gravity.is_some() {
        ctwk_ball.ball_water_gravity = ctwk_ball.ball_water_gravity*ctwk_config.ball_water_gravity.unwrap();
    }
    if ctwk_config.boost_drain_time.is_some() {
        ctwk_ball.boost_drain_time = ctwk_ball.boost_drain_time*ctwk_config.boost_drain_time.unwrap();
    }
    if ctwk_config.boost_min_charge_time.is_some() {
        ctwk_ball.boost_min_charge_time = ctwk_ball.boost_min_charge_time*ctwk_config.boost_min_charge_time.unwrap();
    }
    if ctwk_config.boost_min_rel_speed_for_damage.is_some() {
        ctwk_ball.boost_min_rel_speed_for_damage = ctwk_ball.boost_min_rel_speed_for_damage*ctwk_config.boost_min_rel_speed_for_damage.unwrap();
    }
    if ctwk_config.boost_charge_time0.is_some() {
        ctwk_ball.boost_charge_time0 = ctwk_ball.boost_charge_time0*ctwk_config.boost_charge_time0.unwrap();
    }
    if ctwk_config.boost_charge_time1.is_some() {
        ctwk_ball.boost_charge_time1 = ctwk_ball.boost_charge_time1*ctwk_config.boost_charge_time1.unwrap();
    }
    if ctwk_config.boost_charge_time2.is_some() {
        ctwk_ball.boost_charge_time2 = ctwk_ball.boost_charge_time2*ctwk_config.boost_charge_time2.unwrap();
    }
    if ctwk_config.boost_incremental_speed0.is_some() {
        ctwk_ball.boost_incremental_speed0 = ctwk_ball.boost_incremental_speed0*ctwk_config.boost_incremental_speed0.unwrap();
    }
    if ctwk_config.boost_incremental_speed1.is_some() {
        ctwk_ball.boost_incremental_speed1 = ctwk_ball.boost_incremental_speed1*ctwk_config.boost_incremental_speed1.unwrap();
    }
    if ctwk_config.boost_incremental_speed2.is_some() {
        ctwk_ball.boost_incremental_speed2 = ctwk_ball.boost_incremental_speed2*ctwk_config.boost_incremental_speed2.unwrap();
    }

    Ok(())
}

fn patch_move_item_loss_scan<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
)
-> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer_count = scly.layers.len();
    for i in 0..layer_count {
        let layer = &mut scly.layers.as_mut_vec()[i];
        for obj in layer.objects.as_mut_vec() {
            let mut _poi = obj.property_data.as_point_of_interest_mut();
            if _poi.is_some() {
                let poi = _poi.unwrap();
                poi.position[1] = poi.position[1] + 2.0;
            }
        }
    }

    Ok(())
}

fn patch_remove_control_disabler<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
)
-> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer_count = scly.layers.len();
    for i in 0..layer_count {
        let layer = &mut scly.layers.as_mut_vec()[i];
        for obj in layer.objects.as_mut_vec() {
            let mut _player_hint = obj.property_data.as_player_hint_mut();
            if _player_hint.is_some() {
                let player_hint = _player_hint.unwrap();
                player_hint.inner_struct.unknowns[5] = 0; // always enable unmorphing
                player_hint.inner_struct.unknowns[6] = 0; // always enable morphing
                player_hint.inner_struct.unknowns[7] = 0; // always enable controls
                player_hint.inner_struct.unknowns[8] = 0; // always enable boost
            }
        }
    }

    Ok(())
}

fn patch_add_dock_teleport<'r>(
    ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    source_position: [f32;3],
    source_scale: [f32;3],
    destination_dock_num: u32,
)
-> Result<(), String>
{
    let mrea_id = area.mlvl_area.mrea.to_u32();
    let layer = &mut area.mrea().scly_section_mut().layers.as_mut_vec()[0];
    let spawn_point_id = ps.fresh_instance_id_range.next().unwrap();

    // find the destination dock
    let mut found = false;
    let mut dock_position: GenericArray<f32, U3> = [0.0, 0.0, 0.0].into();
    for obj in layer.objects.as_mut_vec() {
        if !obj.property_data.is_dock() {
            continue;
        }

        let dock = obj.property_data.as_dock().unwrap();
        if dock.dock_index != destination_dock_num {
            continue;
        }
        
        found = true;
        dock_position = dock.position.clone();
    }

    if !found {
        panic!("failed to find dock #{} in room 0x{:X}", destination_dock_num, mrea_id)
    }

    // Find the nearest door
    let mut door_rotation: GenericArray<f32, U3> = [0.0, 0.0, 0.0].into();
    for obj in layer.objects.as_mut_vec() {
        if !obj.property_data.is_door() {
            continue;
        }

        let door = obj.property_data.as_door().unwrap();
        if  f32::abs(door.position[0] - dock_position[0]) > 5.0 ||
            f32::abs(door.position[1] - dock_position[1]) > 5.0 ||
            f32::abs(door.position[2] - dock_position[2]) > 5.0 {
            continue;
        }

        door_rotation = door.rotation.clone();
    }

    let mut spawn_point_position = dock_position.clone();
    let mut spawn_point_rotation = [0.0, 0.0, 0.0];
    let door_offset = 3.0;
    let vertical_offset = -2.0;
    if door_rotation[2] >= 45.0 && door_rotation[2] < 135.0 {
        // Leads North (Y+)
        spawn_point_position[1] = spawn_point_position[1] - door_offset;
        spawn_point_rotation[2] = 180.0;
    } else if (door_rotation[2] >= 135.0 && door_rotation[2] < 225.0) || (door_rotation[2] < -135.0 && door_rotation[2] > -225.0) {
        // Leads East (X+)
        spawn_point_position[0] = spawn_point_position[0] + door_offset;
        spawn_point_rotation[2] = 270.0;
    } else if door_rotation[2] >= -135.0 && door_rotation[2] < -45.0 {
        // Leads South (Y-)
        spawn_point_position[1] = spawn_point_position[1] + door_offset;
        spawn_point_rotation[2] = 0.0;
    } else if door_rotation[2] >= -45.0 && door_rotation[2] < 45.0 {
        // Leads West (X-)
        spawn_point_position[0] = spawn_point_position[0] - door_offset;
        spawn_point_rotation[2] = 90.0;
    }
    spawn_point_position[2] = spawn_point_position[2] + vertical_offset;

    // Insert a spawn point in-bounds
    layer.objects.as_mut_vec().push(
        structs::SclyObject {
            instance_id: spawn_point_id,
            connections: vec![].into(),
            property_data: structs::SclyProperty::SpawnPoint(
                Box::new(structs::SpawnPoint {
                    name: b"dockspawnpoint\0".as_cstr(),
                    position: spawn_point_position.into(),
                    rotation: spawn_point_rotation.into(),
                    power: 0,
                    ice: 0,
                    wave: 0,
                    plasma: 0,
                    missiles: 0,
                    scan_visor: 0,
                    bombs: 0,
                    power_bombs: 0,
                    flamethrower: 0,
                    thermal_visor: 0,
                    charge: 0,
                    super_missile: 0,
                    grapple: 0,
                    xray: 0,
                    ice_spreader: 0,
                    space_jump: 0,
                    morph_ball: 0,
                    combat_visor: 0,
                    boost_ball: 0,
                    spider_ball: 0,
                    power_suit: 0,
                    gravity_suit: 0,
                    varia_suit: 0,
                    phazon_suit: 0,
                    energy_tanks: 0,
                    unknown0: 0,
                    health_refill: 0,
                    unknown1: 0,
                    wavebuster: 0,
                    default_spawn: 0,
                    active: 1,
                    morphed: 0,
                })
            ),
        }
    );

    // Thin out the trigger so that you can't touch it through the door
    let mut thinnest = 0;
    if source_scale[1] < source_scale[thinnest] {
        thinnest = 1;
    }
    if source_scale[2] < source_scale[thinnest] {
        thinnest = 2;
    }
    let mut source_scale = source_scale.clone();
    source_scale[thinnest] = 0.1;

    // Insert a trigger at the previous room which sends the player to the freshly created spawn point
    layer.objects.as_mut_vec().push(
        structs::SclyObject {
            instance_id: ps.fresh_instance_id_range.next().unwrap(),
            connections: vec![
                structs::Connection {
                    state: structs::ConnectionState::ENTERED,
                    message: structs::ConnectionMsg::SET_TO_ZERO,
                    target_object_id: spawn_point_id as u32,
                },
            ].into(),
            property_data: structs::SclyProperty::Trigger(
                Box::new(structs::Trigger {
                    name: b"dockteleporttrigger\0".as_cstr(),
                    position: source_position.into(),
                    scale: source_scale.into(),
                    damage_info: structs::scly_structs::DamageInfo {
                        weapon_type: 0,
                        damage: 0.0,
                        radius: 0.0,
                        knockback_power: 0.0
                    },
                    force: [0.0, 0.0, 0.0].into(),
                    flags: 1,
                    active: 1,
                    deactivate_on_enter: 0,
                    deactivate_on_exit: 0,
                })
            ),
        }
    );

    Ok(())
}

fn patch_modify_dock<'r>(
    _ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    dock_num: u32,
    new_mrea_idx: u32,
)
-> Result<(), String>
{
    let mut replaced = false;
    let mrea_id = area.mlvl_area.mrea.to_u32();
    let attached_areas: &mut reader_writer::LazyArray<'r, u16> = &mut area.mlvl_area.attached_areas;
    let docks: &mut reader_writer::LazyArray<'r, structs::mlvl::Dock<'r>> = &mut area.mlvl_area.docks;

    if dock_num >= attached_areas.as_mut_vec().len() as u32
    {
        panic!("dock num #{} doesn't index attached areas in room 0x{:X}", dock_num, mrea_id);
    }
    let old_mrea_idx = attached_areas.as_mut_vec()[dock_num as usize] as u32;

    // Update the list of attached areas to use the new area instead of the old one
    for i in 0..attached_areas.as_mut_vec().len() {
        if attached_areas.as_mut_vec()[i as usize] == old_mrea_idx as u16 {
            attached_areas.as_mut_vec()[i as usize] = new_mrea_idx as u16;
        }
    }

    // Update dock(s) to load the new area instead of the old one
    for i in 0..docks.as_mut_vec().len() {
        if docks.as_mut_vec()[i].connecting_docks.as_mut_vec()[0].array_index == old_mrea_idx {
            docks.as_mut_vec()[i].connecting_docks.as_mut_vec()[0].array_index = new_mrea_idx;
            replaced = true;
        }
    }

    if !replaced {
        panic!("failed to find mrea idx {} in room 0x{:X} when shuffling rooms", old_mrea_idx, mrea_id);
    }

    Ok(())
}

fn patch_bnr(
    file: &mut structs::FstEntryFile,
    banner: &GameBanner,
)
    -> Result<(), String>
{
    let bnr = match file {
        structs::FstEntryFile::Bnr(bnr) => bnr,
        _ => panic!(),
    };

    bnr.pixels.clone_from_slice(include_bytes!("../extra_assets/banner_image.bin"));

    fn write_encoded_str(field: &str, s: &Option<String>, slice: &mut [u8]) -> Result<(), String>
    {
        if let Some(s) = s {
            let mut bytes = WINDOWS_1252.encode(&s, EncoderTrap::Strict)
                .map_err(|e| format!("Failed to encode banner field {}: {}", field, e))?;
            if bytes.len() >= (slice.len() - 1) {
                Err(format!("Invalid encoded length for banner field {}: expect {}, got {}",
                            field, slice.len() - 1, bytes.len()))?
            }
            bytes.resize(slice.len(), 0u8);
            slice.clone_from_slice(&bytes);
        }
        Ok(())
    }

    write_encoded_str("game_name", &banner.game_name, &mut bnr.english_fields.game_name)?;
    write_encoded_str("developer", &banner.developer, &mut bnr.english_fields.developer)?;
    write_encoded_str(
        "game_name_full",
        &banner.game_name_full,
        &mut bnr.english_fields.game_name_full
    )?;
    write_encoded_str(
        "developer_full",
        &banner.developer_full,
        &mut bnr.english_fields.developer_full)
    ?;
    write_encoded_str("description", &banner.description, &mut bnr.english_fields.description)?;

    Ok(())
}

#[derive(PartialEq, Copy, Clone)]
enum Version
{
    NtscU0_00,
    NtscU0_01,
    NtscU0_02,
    NtscJ,
    Pal,
    NtscUTrilogy,
    NtscJTrilogy,
    PalTrilogy,
}

impl fmt::Display for Version
{
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error>
    {
        match self {
            Version::NtscU0_00    => write!(f, "1.00"),
            Version::NtscU0_01    => write!(f, "1.01"),
            Version::NtscU0_02    => write!(f, "1.02"),
            Version::NtscJ    => write!(f, "jap"),
            Version::Pal         => write!(f, "pal"),
            Version::NtscUTrilogy => write!(f, "trilogy_ntsc_u"),
            Version::NtscJTrilogy => write!(f, "trilogy_ntsc_j"),
            Version::PalTrilogy => write!(f, "trilogy_pal"),
        }
    }
}

fn patch_qol_game_breaking(
    patcher: &mut PrimePatcher,
    version: Version,
    force_vanilla_layout: bool,
    small_samus: bool,
)
{
    // Crashes
    patcher.add_scly_patch(
        resource_info!("07_mines_electric.MREA").into(),
        patch_fix_central_dynamo_crash
    );
    patcher.add_scly_patch(
        resource_info!("00p_mines_connect.MREA").into(),
        patch_fix_pca_crash
    );

    // randomizer-induced bugfixes
    patcher.add_scly_patch(
        resource_info!("1a_morphballtunnel.MREA").into(),
        move |ps, area| patch_spawn_point_position(ps, area, [124.53, -79.78, 22.84], false, false)
    );
    patcher.add_scly_patch(
        resource_info!("05_bathhall.MREA").into(),
        move |ps, area| patch_spawn_point_position(ps, area, [210.512, -82.424, 19.2174], false, false)
    );
    patcher.add_scly_patch(
        resource_info!("00_mines_savestation_b.MREA").into(),
        move |ps, area| patch_spawn_point_position(ps, area, [216.7245, 4.4046, -139.8873], false, true)
    );
    if small_samus {
        patcher.add_scly_patch(
            resource_info!("01_over_mainplaza.MREA").into(),
            move |_ps, area| patch_spawn_point_position(_ps, area, [0.0, 0.0, 0.5], true, false)
        );
        patcher.add_scly_patch(
            resource_info!("0_elev_lava_b.MREA").into(),
            move |_ps, area| patch_spawn_point_position(_ps, area, [0.0, 0.0, 0.7], true, false)
        );
    }

    if force_vanilla_layout { return; }

    // undo retro "fixes"
    if version == Version::NtscU0_00 {
        patcher.add_scly_patch(
            resource_info!("00n_ice_connect.MREA").into(),
            patch_research_core_access_soft_lock
        );
    } else {
        patcher.add_scly_patch(
            resource_info!("08_courtyard.MREA").into(),
            patch_arboretum_invisible_wall
        );
        if version != Version::NtscU0_01 {
            patcher.add_scly_patch(
                resource_info!("05_ice_shorelines.MREA").into(),
                move |ps, area| patch_ruined_courtyard_thermal_conduits(ps, area, version)
            );
        }
    }
    if version == Version::NtscU0_02 {
        patcher.add_scly_patch(
            resource_info!("01_mines_mainplaza.MREA").into(),
            patch_main_quarry_door_lock_0_02
        );
        patcher.add_scly_patch(
            resource_info!("13_over_burningeffigy.MREA").into(),
            patch_geothermal_core_door_lock_0_02
        );
        patcher.add_scly_patch(
            resource_info!("19_hive_totem.MREA").into(),
            patch_hive_totem_boss_trigger_0_02
        );
    }
    if version == Version::Pal || version == Version::NtscJ || version == Version::NtscUTrilogy || version == Version::NtscJTrilogy || version == Version::PalTrilogy {
        patcher.add_scly_patch(
            resource_info!("04_mines_pillar.MREA").into(),
            patch_ore_processing_destructible_rock_pal
        );
        patcher.add_scly_patch(
            resource_info!("13_over_burningeffigy.MREA").into(),
            patch_geothermal_core_destructible_rock_pal
        );
        if version == Version::Pal {
            patcher.add_scly_patch(
                resource_info!("01_mines_mainplaza.MREA").into(),
                patch_main_quarry_door_lock_pal
            );
        }
    }

    // softlocks
    patcher.add_scly_patch(
        resource_info!("22_Flaahgra.MREA").into(),
        patch_sunchamber_prevent_wild_before_flaahgra
    );
    patcher.add_scly_patch(
        resource_info!("0v_connect_tunnel.MREA").into(),
        patch_sun_tower_prevent_wild_before_flaahgra
    );
    patcher.add_scly_patch(
        resource_info!("13_ice_vault.MREA").into(),
        patch_research_lab_aether_exploding_wall
    );
    patcher.add_scly_patch(
        resource_info!("11_ice_observatory.MREA").into(),
        patch_observatory_2nd_pass_solvablility
    );
    patcher.add_scly_patch(
        resource_info!("11_ice_observatory.MREA").into(),
        patch_observatory_1st_pass_softlock
    );
    patcher.add_scly_patch(
        resource_info!("02_mines_shotemup.MREA").into(),
        patch_mines_security_station_soft_lock
    );
    patcher.add_scly_patch(
        resource_info!("18_ice_gravity_chamber.MREA").into(),
        patch_gravity_chamber_stalactite_grapple_point
    );
}

fn patch_qol_logical(patcher: &mut PrimePatcher, config: &PatchConfig)
{
    if config.main_plaza_door {      
        patcher.add_scly_patch(
            resource_info!("01_mainplaza.MREA").into(),
            make_main_plaza_locked_door_two_ways
        );
    }

    if config.phazon_elite_without_dynamo {
        make_elite_research_fight_prereq_patches(patcher);
    }

    if config.backwards_frigate {
        patcher.add_scly_patch(
            resource_info!("08b_under_intro_ventshaft.MREA").into(),
            patch_main_ventilation_shaft_section_b_door
        );
    }

    if config.backwards_labs {        
        patcher.add_scly_patch(
            resource_info!("10_ice_research_a.MREA").into(),
            patch_research_lab_hydra_barrier
        );
    }
    
    if config.backwards_upper_mines {
        patcher.add_scly_patch(
            resource_info!("01_mines_mainplaza.MREA").into(),
            patch_main_quarry_barrier
        );
    }

    if config.backwards_lower_mines {
        patcher.add_scly_patch(
            resource_info!("00p_mines_connect.MREA").into(),
            patch_backwards_lower_mines_pca
        );
        patcher.add_scly_patch(
            resource_info!("00o_mines_connect.MREA").into(),
            patch_backwards_lower_mines_eqa
        );
        patcher.add_scly_patch(
            resource_info!("11_mines.MREA").into(),
            patch_backwards_lower_mines_mqb
        );
        patcher.add_scly_patch(
            resource_info!("08_mines.MREA").into(),
            patch_backwards_lower_mines_mqa
        );
        patcher.add_scly_patch(
            resource_info!("05_mines_forcefields.MREA").into(),
            patch_backwards_lower_mines_elite_control
        );
    }
}

fn patch_qol_cosmetic(
    patcher: &mut PrimePatcher,
    skip_ending_cinematic: bool,
)
{
    // Replace the attract mode FMVs with empty files to reduce the amount of data we need to
    // copy and to make compressed ISOs smaller.
    const FMV_NAMES: &[&[u8]] = &[
        b"Video/attract0.thp",
        b"Video/attract1.thp",
        b"Video/attract2.thp",
        b"Video/attract3.thp",
        b"Video/attract4.thp",
        b"Video/attract5.thp",
        b"Video/attract6.thp",
        b"Video/attract7.thp",
        b"Video/attract8.thp",
        b"Video/attract9.thp",
    ];
    const FMV: &[u8] = include_bytes!("../extra_assets/attract_mode.thp");
    for name in FMV_NAMES {
        patcher.add_file_patch(name, |file| {
            *file = structs::FstEntryFile::ExternalFile(Box::new(FMV));
            Ok(())
        });
    }

    patcher.add_resource_patch(
        resource_info!("FRME_BallHud.FRME").into(),
        patch_morphball_hud,
    );

    if skip_ending_cinematic {
        patcher.add_scly_patch(
            resource_info!("01_endcinema.MREA").into(),
            patch_ending_scene_straight_to_credits
        );
    }

    // not shown here - hudmemos are nonmodal and item aquisition cutscenes are removed
}

fn patch_qol_competitive_cutscenes(patcher: &mut PrimePatcher, version: Version) {
    patcher.add_scly_patch(
        resource_info!("01_mines_mainplaza.MREA").into(), // main quarry (just pirate booty)
        move |ps, area| patch_remove_cutscenes(ps, area,
            vec![],
            vec![
                0x000203DE, 0x000203DC, 0x0002040D, 0x0002040C, // keep area entrance cutscene
                0x0002023E, 0x00020021, 0x00020253, // keep crane cutscenes
                0x0002043D, // keep barrier cutscene
            ],
            false,
        ),
    );
    patcher.add_scly_patch(
        resource_info!("08_courtyard.MREA").into(), // Arboretum
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x0013012E, 0x00130131, 0x00130141], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("10_over_1alavaarea.MREA").into(), // magmoor workstation
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![0x00170153], false), // skip patching 1st cutscene (special floaty case)
    );
    patcher.add_scly_patch(
        resource_info!("05_over_xray.MREA").into(), // life grove (competitive only - watch raise post cutscenes)
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![0x002A00C4, 0x002A01D0], false), // don't skip ghosts cutscene or ghosts will leave forever
    );
    patcher.add_scly_patch(
        resource_info!("12_ice_research_b.MREA").into(),
        move |ps, area| patch_lab_aether_cutscene_trigger(ps, area, version)
    );
    patcher.add_scly_patch(
        resource_info!("00j_over_hall.MREA").into(), // temple security station
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("15_ice_cave_a.MREA").into(), // frost cave
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x0029006C, 0x0029006B], vec![], true),
    );
    patcher.add_scly_patch(
        resource_info!("15_energycores.MREA").into(), // energy core
        move |ps, area| patch_remove_cutscenes(ps, area,
            vec![
                0x002C00E8, 0x002C0101, 0x002C00F5, // activate core delay
                0x002C0068, 0x002C0055, 0x002C0079, // core energy flow activation delay
                0x002C0067, 0x002C00E7, 0x002C0102, // jingle finish delay
                0x002C0104, 0x002C00EB, // platform go up delay
                0x002C0069, // water go down delay
                0x002C01BC, // unlock door
            ],
            vec![],
            false,
        ),
    );
    patcher.add_scly_patch(
        resource_info!("07_under_intro_reactor.MREA").into(), // reactor core
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("06_under_intro_freight.MREA").into(), // cargo freight lift
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x001B0100], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("05_under_intro_zoo.MREA").into(), // biohazard containment
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x001E028A], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("05_under_intro_specimen_chamber.MREA").into(), // biotech research area 1
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x002000DB], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("04_maproom_d.MREA").into(), // vault
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("0v_connect_tunnel.MREA").into(), // sun tower
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x001D00E5, 0x001D00E8], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("07_ruinedroof.MREA").into(), // training chamber
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x000C0153, 0x000C0154, 0x000C015B], vec![], true),
    );
    patcher.add_scly_patch(
        resource_info!("11_wateryhall.MREA").into(), // watery hall
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x0029280A, 0x002927FD], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("18_halfpipe.MREA").into(), // crossway
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("13_over_burningeffigy.MREA").into(), // geothermal core
        move |ps, area| patch_remove_cutscenes(ps, area,
            vec![0x001401DD, 0x001401E3], // immediately move parts
            vec![],
            false,
        ),
    );
    patcher.add_scly_patch(
        resource_info!("06_ice_temple.MREA").into(), // chozo ice temple
        move |ps, area| patch_remove_cutscenes(ps, area,
            vec![0x00080201, 0x0008024E,0x00080170, 0x00080118], // speed up hands animation + grate open
            vec![],
            false),
    );
    patcher.add_scly_patch(
        resource_info!("04_ice_boost_canyon.MREA").into(), // Phendrana canyon
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("05_ice_shorelines.MREA").into(), // ruined courtyard
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("13_ice_vault.MREA").into(), // research core
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("03_mines.MREA").into(), // elite research (keep phazon elite cutscene)
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![0x000D04C8, 0x000D01CF], false),
    );
    patcher.add_scly_patch(
        resource_info!("02_mines_shotemup.MREA").into(), // mine security station
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], true),
    );
}

fn patch_qol_minor_cutscenes(patcher: &mut PrimePatcher, version: Version) {
    patcher.add_scly_patch(
        resource_info!("08_courtyard.MREA").into(), // Arboretum
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x0013012E, 0x00130131, 0x00130141], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("08_mines.MREA").into(), // MQA (just first cutscene)
        move |ps, area| patch_remove_cutscenes(ps, area,
            vec![],
            vec![0x002000CF], // 2nd cutscene
            false,
        ),
    );
    patcher.add_scly_patch(
        resource_info!("12_ice_research_b.MREA").into(),
        move |ps, area| patch_lab_aether_cutscene_trigger(ps, area, version)
    );
    patcher.add_scly_patch(
        resource_info!("00j_over_hall.MREA").into(), // temple security station
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("15_ice_cave_a.MREA").into(), // frost cave
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x0029006C, 0x0029006B], vec![], true),
    );
    patcher.add_scly_patch(
        resource_info!("15_energycores.MREA").into(), // energy core
        move |ps, area| patch_remove_cutscenes(ps, area,
            vec![
                0x002C00E8, 0x002C0101, 0x002C00F5, // activate core delay
                0x002C0068, 0x002C0055, 0x002C0079, // core energy flow activation delay
                0x002C0067, 0x002C00E7, 0x002C0102, // jingle finish delay
                0x002C0104, 0x002C00EB, // platform go up delay
                0x002C0069, // water go down delay
                0x002C01BC, // unlock door
            ],
            vec![],
            false,
        ),
    );
    patcher.add_scly_patch(
        resource_info!("10_over_1alavaarea.MREA").into(), // magmoor workstation
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![0x00170153], false), // skip patching 1st cutscene (special floaty case)
    );
    patcher.add_scly_patch(
        resource_info!("07_under_intro_reactor.MREA").into(), // reactor core
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("06_under_intro_freight.MREA").into(), // cargo freight lift
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x001B0100], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("05_under_intro_zoo.MREA").into(), // biohazard containment
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x001E028A], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("05_under_intro_specimen_chamber.MREA").into(), // biotech research area 1
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x002000DB], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("05_over_xray.MREA").into(), // life grove
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![0x002A00C4], false), // don't skip ghosts cutscene or ghosts will leave forever
    );
    patcher.add_scly_patch(
        resource_info!("01_mainplaza.MREA").into(), // main plaza
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("01_mines_mainplaza.MREA").into(), // main quarry
        move |ps, area| patch_remove_cutscenes(ps, area,
            vec![0x00020443], // turn the forcefield off faster
            vec![],
            false,
        ),
    );
    patcher.add_scly_patch(
        resource_info!("11_over_muddywaters_b.MREA").into(), // lava lake
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("04_maproom_d.MREA").into(), // vault
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("0v_connect_tunnel.MREA").into(), // sun tower
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x001D00E5, 0x001D00E8], vec![], false), // Open gate faster
    );
    patcher.add_scly_patch(
        resource_info!("07_ruinedroof.MREA").into(), // training chamber
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x000C0153, 0x000C0154, 0x000C015B], vec![], true),
    );
    patcher.add_scly_patch(
        resource_info!("11_wateryhall.MREA").into(), // watery hall
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x0029280A, 0x002927FD], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("18_halfpipe.MREA").into(), // crossway
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("17_chozo_bowling.MREA").into(), // hall of the elders
        move |ps, area| patch_remove_cutscenes(ps, area,
            vec![0x003400F4, 0x003400F8, 0x003400F9, 0x0034018C], // speed up release from bomb slots
            vec![
                0x003400F5, 0x00340046, 0x0034004A, 0x003400EA, 0x0034004F, // leave chozo bowling cutscenes to avoid getting stuck
                0x0034025C, 0x00340264, 0x00340268, 0x0034025B, // leave missile station cutsene
                0x00340142, 0x00340378, // leave ghost death cutscene (it's major b/c reposition)
            ],
            false,
        ),
    );
    patcher.add_scly_patch(
        resource_info!("13_over_burningeffigy.MREA").into(), // geothermal core
        move |ps, area| patch_remove_cutscenes(ps, area,
            vec![0x001401DD, 0x001401E3], // immediately move parts
            vec![],
            false,
        ),
    );
    patcher.add_scly_patch(
        resource_info!("00h_mines_connect.MREA").into(), // vent shaft
        move |ps, area| patch_remove_cutscenes(ps, area,
            vec![0x001200C3, 0x001200DE], // activate gas faster
            vec![],
            true,
        ),
    );
    patcher.add_scly_patch(
        resource_info!("06_ice_temple.MREA").into(), // chozo ice temple
        move |ps, area| patch_remove_cutscenes(ps, area,
            vec![0x00080201, 0x0008024E,0x00080170, 0x00080118], // speed up hands animation + grate open
            vec![],
            false),
    );
    patcher.add_scly_patch(
        resource_info!("04_ice_boost_canyon.MREA").into(), // Phendrana canyon
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("05_ice_shorelines.MREA").into(), // ruined courtyard
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("11_ice_observatory.MREA").into(), // Observatory
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x001E0042, 0x001E000E], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("08_ice_ridley.MREA").into(), // control tower
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x002702DD, 0x002702D5, 0x00270544, 0x002703DF], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("13_ice_vault.MREA").into(), // research core
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("03_mines.MREA").into(), // elite research (keep phazon elite cutscene)
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![0x000D04C8, 0x000D01CF], false),
    );
    patcher.add_scly_patch(
        resource_info!("06_mines_elitebustout.MREA").into(), // omega reserach
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], true),
    );
    patcher.add_scly_patch(
        resource_info!("07_mines_electric.MREA").into(), // central dynamo
        move |ps, area| patch_remove_cutscenes(ps, area,
            vec![0x001B03F8], // activate maze faster
            vec![0x001B0349, 0x001B0356], // keep item aquisition cutscene (or players can get left down there)
            true,
        ),
    );
    patcher.add_scly_patch(
        resource_info!("02_mines_shotemup.MREA").into(), // mine security station
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], true),
    );
    patcher.add_scly_patch(
        resource_info!("01_ice_plaza.MREA").into(), // phendrana shorelines
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x00020203], vec![0x000202A9, 0x000202A8, 0x000202B7], true), // keep the ridley cinematic
    );
}

pub fn patch_qol_major_cutscenes(patcher: &mut PrimePatcher) {
    patcher.add_scly_patch(
        resource_info!("08_courtyard.MREA").into(), // Arboretum
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x0013012E, 0x00130131, 0x00130141], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("01_endcinema.MREA").into(), // Impact Crater Escape Cinema (cause why not)
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], true),
    );
    // +Ghost death cutscene
    patcher.add_scly_patch(
        resource_info!("17_chozo_bowling.MREA").into(), // hall of the elders
        move |ps, area| patch_remove_cutscenes(ps, area,
            vec![0x003400F4, 0x003400F8, 0x003400F9, 0x0034018C], // speed up release from bomb slots
            vec![
                0x003400F5, 0x00340046, 0x0034004A, 0x003400EA, 0x0034004F, // leave chozo bowling cutscenes to avoid getting stuck
                0x0034025C, 0x00340264, 0x00340268, 0x0034025B, // leave missile station cutsene
            ],
            false,
        ),
    );
    patcher.add_scly_patch(
        resource_info!("01_ice_plaza.MREA").into(), // phendrana shorelines
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x00020203], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("07_stonehenge.MREA").into(), // artifact temple
        move |ps, area| patch_remove_cutscenes(ps, area,
            vec![],
            vec![
                // progress cutscene
                0x00100463, 0x0010046F,
                // ridley intro cutscene
                0x0010036F, 0x0010026C, 0x00100202, 0x00100207, 0x00100373, 0x001003C4, 0x001003D9, 0x001003DC, 0x001003E6, 0x001003CE, 0x0010020C, 0x0010021A, 0x001003EF, 0x001003E9, 0x0010021A, 0x00100491, 0x001003EE, 0x001003F0, 0x001003FE, 0x0010021F,
                // crater entry/exit cutscene
                0x001002C8, 0x001002B8, 0x001002C2,
            ],
            true,
        ),
    );
    patcher.add_scly_patch(
        resource_info!("03_mines.MREA").into(), // elite research
        move |ps, area| patch_remove_cutscenes(ps, area, vec![0x000D01A9], vec![], true),
    );
    patcher.add_scly_patch(
        resource_info!("19_hive_totem.MREA").into(), // hive totem
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("1a_morphball_shrine.MREA").into(), // ruined shrine
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], true),
    );
    patcher.add_scly_patch(
        resource_info!("03_monkey_lower.MREA").into(), // burn dome
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], true),
    );
    patcher.add_scly_patch(
        resource_info!("22_Flaahgra.MREA").into(), // sunchamber
        move |ps, area| patch_remove_cutscenes(
            ps, area,
            vec![
                0x00250092, 0x00250093, 0x00250094, 0x002500A8, // release from bomb slot
                0x0025276A, // acid --> water (needed for floaty)
            ],
            vec![
                0x002500CA, 0x00252FE4, 0x00252727, 0x0025272C, 0x00252741,  // into cinematic works better if skipped normally
                0x0025000B, // you get put in vines timeout if you skip the first reposition:
                            // https://cdn.discordapp.com/attachments/761000402182864906/840707140364664842/no-spawnpoints.mp4
                0x00250123, // keep just the first camera angle of the death cutscene to prevent underwater when going for pre-floaty
                0x00252FC0, // the last reposition is important for floaty jump
            ],
            false,
        ),
    );
    patcher.add_scly_patch(
        resource_info!("07_ice_chapel.MREA").into(), // chapel of the elders
        move |ps, area| patch_remove_cutscenes(ps, area,
            vec![0x000E0057], // Faster adult breakout
            vec![0x000E019D, 0x000E019B], // keep fight start reposition for wavesun
            true,
        ),
    );
    patcher.add_scly_patch(
        resource_info!("09_ice_lobby.MREA").into(), // research entrance
        move |ps, area| patch_remove_cutscenes(ps, area,
            vec![0x001402F7, 0x00140243, 0x001402D6, 0x001402D0, 0x001402B3], // start fight faster
            vec![],
            false
        ),
    );
    patcher.add_scly_patch(
        resource_info!("19_ice_thardus.MREA").into(), // Quarantine Cave
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], true),
    );
    patcher.add_scly_patch(
        resource_info!("05_mines_forcefields.MREA").into(), // elite control
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], true),
    );
    patcher.add_scly_patch(
        resource_info!("08_mines.MREA").into(), // MQA
        move |ps, area| patch_remove_cutscenes(ps, area,
            vec![
                0x002000D7, // Timer_pikeend
                0x002000DE, // Timer_coverstart
                0x002000E0, // Timer_steamshutoff
                0x00200708, // Timer - Shield Off, Play Battle Music
            ],
            vec![],
            false,
        ),
    );
    patcher.add_scly_patch(
        resource_info!("12_mines_eliteboss.MREA").into(), // elite quarters
        move |ps, area| patch_remove_cutscenes(
            ps, area, vec![],
            vec![ // keep the first cutscene because the normal skip works out better
                0x001A0282, 0x001A0283, 0x001A02B3, 0x001A02BF, 0x001A0284, 0x001A031A, // cameras
                0x001A0294, 0x001A02B9, // player actor
            ],
            true,
        ),
    );
    patcher.add_scly_patch( // phazon infusion chamber
        resource_info!("03a_crater.MREA").into(),
        move |ps, area| patch_remove_cutscenes(
            ps, area, vec![],
            vec![ // keep first cutscene because vanilla skip is better
                0x0005002B, 0x0005002C, 0x0005007D, 0x0005002D, 0x00050032, 0x00050078, 0x00050033, 0x00050034, 0x00050035, 0x00050083, // cameras
                0x0005002E, 0x0005008B, 0x00050089, // player actors
            ],
            false,
        ),
    );

    // subchambers 1-4 (see special handling for exo aggro)
    patcher.add_scly_patch(
        resource_info!("03b_crater.MREA").into(),
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("03c_crater.MREA").into(),
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("03d_crater.MREA").into(),
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], false),
    );
    patcher.add_scly_patch(
        resource_info!("03e_crater.MREA").into(),
        move |ps, area| patch_remove_cutscenes(ps, area, vec![], vec![], true),
    );

    // play subchamber 5 cutscene normally (players can't natrually pass through the ceiling of prime's lair)

    patcher.add_scly_patch(
        resource_info!("03f_crater.MREA").into(), // metroid prime lair
        move |ps, area| patch_remove_cutscenes(
            ps, area, vec![],
            vec![ // play the first cutscene so it can be skipped normally
                0x000B019D, 0x000B008B, 0x000B008D, 0x000B0093, 0x000B0094, 0x000B00A7,
                0x000B00AF, 0x000B00E1, 0x000B00DF, 0x000B00B0, 0x000B00D3, 0x000B00E3,
                0x000B00E6, 0x000B0095, 0x000B00E4,
            ],
            true,
        ),
    );
}

fn patch_power_conduits<'a>(patcher: &mut PrimePatcher<'_, 'a>)
{
    patcher.add_scly_patch(
        resource_info!("05_ice_shorelines.MREA").into(), // ruined courtyard
        patch_thermal_conduits_damage_vulnerabilities
    );

    patcher.add_scly_patch(
        resource_info!("13_ice_vault.MREA").into(), // research core
        patch_thermal_conduits_damage_vulnerabilities
    );
    
    patcher.add_scly_patch(
        resource_info!("08b_under_intro_ventshaft.MREA").into(), // Main Ventilation Shaft Section B
        patch_thermal_conduits_damage_vulnerabilities
    );

    patcher.add_scly_patch(
        resource_info!("07_under_intro_reactor.MREA").into(), // reactor core
        patch_thermal_conduits_damage_vulnerabilities
    );
    
    patcher.add_scly_patch(
        resource_info!("06_under_intro_to_reactor.MREA").into(), // reactor core access
        patch_thermal_conduits_damage_vulnerabilities
    );
    
    patcher.add_scly_patch(
        resource_info!("06_under_intro_freight.MREA").into(), // cargo freight lift to deck gamma
        patch_thermal_conduits_damage_vulnerabilities
    );
    
    patcher.add_scly_patch(
        resource_info!("05_under_intro_zoo.MREA").into(), // biohazard containment
        patch_thermal_conduits_damage_vulnerabilities
    );
    
    patcher.add_scly_patch(
        resource_info!("05_under_intro_specimen_chamber.MREA").into(), // biotech research area 1
        patch_thermal_conduits_damage_vulnerabilities
    );
    
    patcher.add_scly_patch(
        resource_info!("01_mines_mainplaza.MREA").into(), // main quarry
        patch_thermal_conduits_damage_vulnerabilities
    );

    patcher.add_scly_patch(
        resource_info!("10_over_1alavaarea.MREA").into(), // magmoor workstation
        patch_thermal_conduits_damage_vulnerabilities
    );
}

pub fn patch_iso<T>(config: PatchConfig, mut pn: T) -> Result<(), String>
    where T: structs::ProgressNotifier
{
    let mut ct = Vec::new();
    writeln!(ct, "Created by randomprime version {}", env!("CARGO_PKG_VERSION")).unwrap();
    writeln!(ct).unwrap();
    writeln!(ct, "Options used:").unwrap();
    writeln!(ct, "qol game breaking: {:?}", config.qol_game_breaking).unwrap();
    writeln!(ct, "qol cosmetic: {:?}", config.qol_cosmetic).unwrap();
    writeln!(ct, "nonvaria heat damage: {}", config.nonvaria_heat_damage).unwrap();
    writeln!(ct, "heat damage per sec: {}", config.heat_damage_per_sec).unwrap();
    writeln!(ct, "staggered suit damage: {}", config.staggered_suit_damage).unwrap();
    writeln!(ct, "etank capacity: {}", config.etank_capacity).unwrap();
    writeln!(ct, "map default state: {}", config.map_default_state.to_string().to_lowercase()).unwrap();
    for (pickup_type, value) in &config.item_max_capacity {
        writeln!(ct, "{} capacity: {}", pickup_type.name(), value).unwrap();
    }
    writeln!(ct, "{}", config.comment).unwrap();

    let mut reader = Reader::new(&config.input_iso[..]);

    let mut gc_disc: structs::GcDisc = reader.read(());

    let version = match (&gc_disc.header.game_identifier(), gc_disc.header.disc_id, gc_disc.header.version) {
        (b"GM8E01", 0, 0) => Version::NtscU0_00,
        (b"GM8E01", 0, 1) => Version::NtscU0_01,
        (b"GM8E01", 0, 2) => Version::NtscU0_02,
        (b"GM8J01", 0, 0) => Version::NtscJ,
        (b"GM8P01", 0, 0) => Version::Pal,
        (b"R3ME01", 0, 0) => Version::NtscUTrilogy,
        (b"R3IJ01", 0, 0) => Version::NtscJTrilogy,
        (b"R3MP01", 0, 0) => Version::PalTrilogy,
        _ => Err(concat!(
                "The input ISO doesn't appear to be NTSC-US, PAL Metroid Prime, ",
                "or NTSC-US, NTSC-J, PAL Metroid Prime Trilogy."
            ))?
    };
    if gc_disc.find_file("randomprime.txt").is_some() {
        Err(concat!("The input ISO has already been randomized once before. ",
                    "You must start from an unmodified ISO every time."
        ))?
    }
    if version == Version::NtscU0_01 {
        Err("The NTSC 0-01 version of Metroid Prime is not current supported.")?;
    }

    build_and_run_patches(&mut gc_disc, &config, version)?;

    gc_disc.add_file("randomprime.txt", structs::FstEntryFile::Unknown(Reader::new(&ct)))?;


    let patches_rel_bytes = match version {
        Version::NtscU0_00    => Some(rel_files::PATCHES_100_REL),
        Version::NtscU0_01    => None,
        Version::NtscU0_02    => Some(rel_files::PATCHES_102_REL),
        Version::Pal          => Some(rel_files::PATCHES_PAL_REL),
        Version::NtscJ        => None,
        Version::NtscUTrilogy => None,
        Version::NtscJTrilogy => None,
        Version::PalTrilogy => None,
    };
    if let Some(patches_rel_bytes) = patches_rel_bytes {
        gc_disc.add_file(
            "patches.rel",
            structs::FstEntryFile::Unknown(Reader::new(patches_rel_bytes))
        )?;
    }

    match config.iso_format {
        IsoFormat::Iso => {
            let mut file = config.output_iso;
            file.set_len(structs::GC_DISC_LENGTH as u64)
                .map_err(|e| format!("Failed to resize output file: {}", e))?;
            gc_disc.write(&mut file, &mut pn)
                .map_err(|e| format!("Error writing output file: {}", e))?;
            pn.notify_flushing_to_disk();
        },
        IsoFormat::Gcz => {
            let mut gcz_writer = GczWriter::new(config.output_iso, structs::GC_DISC_LENGTH as u64)
                .map_err(|e| format!("Failed to prepare output file for writing: {}", e))?;
            gc_disc.write(&mut *gcz_writer, &mut pn)
                .map_err(|e| format!("Error writing output file: {}", e))?;
            pn.notify_flushing_to_disk();
        },
        IsoFormat::Ciso => {
            let mut ciso_writer = CisoWriter::new(config.output_iso)
                .map_err(|e| format!("Failed to prepare output file for writing: {}", e))?;
            gc_disc.write(&mut ciso_writer, &mut pn)
                .map_err(|e| format!("Error writing output file: {}", e))?;
            pn.notify_flushing_to_disk();
        }
    };
    Ok(())
}

fn build_and_run_patches(gc_disc: &mut structs::GcDisc, config: &PatchConfig, version: Version)
    -> Result<(), String>
{
    let remove_ball_color = config.ctwk_config.morph_ball_size.clone().unwrap_or(1.0) < 0.999;
    let remove_control_disabler = config.ctwk_config.player_size.clone().unwrap_or(1.0) < 0.999 || config.ctwk_config.morph_ball_size.clone().unwrap_or(1.0) < 0.999;
    let move_item_loss_scan = config.ctwk_config.player_size.clone().unwrap_or(1.0) > 1.001;

    let starting_room = SpawnRoomData::from_str(&config.starting_room);

    let frigate_done_room = {
        let mut destination_name = "Tallon:Landing Site";
        let frigate_level = config.level_data.get(World::FrigateOrpheon.to_json_key());
        if frigate_level.is_some() {
            let x = frigate_level.unwrap().transports.get(&"Frigate Escape Cutscene".to_string());
            if x.is_some() {
                destination_name = x.unwrap();
            }
        }

        SpawnRoomData::from_str(destination_name)
    };
    let essence_done_room = {
        let mut destination = None;
        let crater_level = config.level_data.get(World::ImpactCrater.to_json_key());
        if crater_level.is_some() {
            let x = crater_level.unwrap().transports.get(&"Essence Dead Cutscene".to_string());
            if x.is_some() {
                destination = Some(SpawnRoomData::from_str(x.unwrap()))
            }
        }

        destination        
    };

    let mut rng = StdRng::seed_from_u64(config.seed);
    let artifact_totem_strings = build_artifact_temple_totem_scan_strings(config, &mut rng, config.artifact_hints.clone());

    let show_starting_memo = config.starting_memo.is_some();

    let starting_memo = {
        if config.starting_memo.is_some() {
            Some(config.starting_memo.as_ref().unwrap().as_str())
        } else {
            None
        }
    };

    let (game_resources, pickup_hudmemos, pickup_scans, extra_scans, savw_scans_to_add, extern_models) =
        collect_game_resources(gc_disc, starting_memo, &config)?;

    let extern_models = &extern_models;
    let game_resources = &game_resources;
    let pickup_hudmemos = &pickup_hudmemos;
    let pickup_scans = &pickup_scans;
    let extra_scans = &extra_scans;
    let savw_scans_to_add = &savw_scans_to_add;

    // XXX These values need to out live the patcher
    let select_game_fmv_suffix = ["A", "B", "C"].choose(&mut rng).unwrap();
    let n = format!("Video/02_start_fileselect_{}.thp", select_game_fmv_suffix);
    let start_file_select_fmv = gc_disc.find_file(&n).unwrap().file().unwrap().clone();
    let n = format!("Video/04_fileselect_playgame_{}.thp", select_game_fmv_suffix);
    let file_select_play_game_fmv = gc_disc.find_file(&n).unwrap().file().unwrap().clone();

    let mut patcher = PrimePatcher::new();

    patcher.add_file_patch(b"opening.bnr", |file| patch_bnr(file, &config.game_banner));

    if let Some(flaahgra_music_files) = &config.flaahgra_music_files {
        const MUSIC_FILE_NAME: &[&[u8]] = &[
            b"Audio/rui_flaaghraR.dsp",
            b"Audio/rui_flaaghraL.dsp",
        ];
        for (file_name, music_file) in MUSIC_FILE_NAME.iter().zip(flaahgra_music_files.iter()) {
            patcher.add_file_patch(file_name, move |file| {
                *file = structs::FstEntryFile::ExternalFile(Box::new(music_file.clone()));
                Ok(())
            });
        }
    }

    // Patch Tweaks.pak
    patcher.add_resource_patch(
        resource_info!("Game.CTWK").into(),
        |res| patch_ctwk_game(res, &config.ctwk_config),
    );
    patcher.add_resource_patch(
        resource_info!("Player.CTWK").into(),
        |res| patch_ctwk_player(res, &config.ctwk_config),
    );
    patcher.add_resource_patch(
        resource_info!("PlayerGun.CTWK").into(),
        |res| patch_ctwk_player_gun(res, &config.ctwk_config),
    );
    patcher.add_resource_patch(
        resource_info!("Ball.CTWK").into(),
        |res| patch_ctwk_ball(res, &config.ctwk_config),
    );

    /* TODO: add more tweaks
    953a7c63.CTWK -> Game.CTWK
    264a4972.CTWK -> Player.CTWK
    f1ed8fd7.CTWK -> PlayerControls.CTWK
    3faec012.CTWK -> PlayerControls2.CTWK
    85ca11e9.CTWK -> PlayerRes.CTWK
    6907a32d.CTWK -> PlayerGun.CTWK
    33b3323a.CTWK -> GunRes.CTWK
    5ed56350.CTWK -> Ball.CTWK
    94c76ecd.CTWK -> Targeting.CTWK
    39ad28d3.CTWK -> CameraBob.CTWK
    5f24eff8.CTWK -> SlideShow.CTWK
    ed2e48a9.CTWK -> Gui.CTWK
    c9954e56.CTWK -> GuiColors.CTWK
    e66a4f86.CTWK -> AutoMapper.CTWK
    1d180d7c.CTWK -> Particle.CTWK
    */

    // Patch end sequence (player size)
    if config.ctwk_config.player_size.is_some() {
        patcher.add_scly_patch(
            resource_info!("01_endcinema.MREA").into(),
            move |ps, area| patch_samus_actor_size(ps, area, config.ctwk_config.player_size.clone().unwrap()),
        );
    }

    // Add hard-coded POI
    if config.qol_pickup_scans {
        patcher.add_scly_patch(
            resource_info!("01_ice_plaza.MREA").into(), // Phen Shorelines - Scannable in tower
            move |ps, area| patch_add_poi(
                ps, area,
                game_resources,
                custom_asset_ids::SHORELINES_POI_SCAN,
                custom_asset_ids::SHORELINES_POI_STRG,
                [-98.0624, -162.3933, 28.5371],
            ),
        );
        patcher.add_scly_patch(
            resource_info!("08_mines.MREA").into(), // MQA - Always scan dash from item
            move |ps, area| patch_add_poi(
                ps, area,
                game_resources,
                custom_asset_ids::MQA_POI_SCAN,
                custom_asset_ids::MQA_POI_STRG,
                [224.9169, 255.7093, -67.2823],
            ),
        );
    }
    patcher.add_scly_patch(
        resource_info!("06_under_intro_freight.MREA").into(),
        move |ps, area| patch_add_poi(
            ps, area,
            game_resources,
            custom_asset_ids::CFLDG_POI_SCAN,
            custom_asset_ids::CFLDG_POI_STRG,
            [-44.0, 361.0, -120.0],
        ),
    );
    
    if config.qol_cutscenes == CutsceneMode::Competitive {
        patch_qol_competitive_cutscenes(&mut patcher, version);
    }

    if config.qol_cutscenes == CutsceneMode::Minor || config.qol_cutscenes == CutsceneMode::Major {
        patch_qol_minor_cutscenes(&mut patcher, version);
    }

    if config.qol_cutscenes == CutsceneMode::Major {
        patch_qol_major_cutscenes(&mut patcher);
    }

    // Patch pickups
    for (pak_name, rooms) in pickup_meta::ROOM_INFO.iter() {
        let world = World::from_pak(pak_name).unwrap();

        for room_info in rooms.iter() {
            if remove_control_disabler {
                patcher.add_scly_patch(
                    (pak_name.as_bytes(), room_info.room_id.to_u32()),
                    patch_remove_control_disabler,
                );
            }

            if config.ctwk_config.player_size.is_some() {
                patcher.add_scly_patch(
                    (pak_name.as_bytes(), room_info.room_id.to_u32()),
                    move |ps, area| patch_samus_actor_size(ps, area, config.ctwk_config.player_size.clone().unwrap()),
                );
            }

            if config.force_vanilla_layout {continue;}

            // Remove objects patch
            if config.qol_cosmetic {
                patcher.add_scly_patch((pak_name.as_bytes(), room_info.room_id.to_u32()), move |_, area| {
                    let layers = area.mrea().scly_section_mut().layers.as_mut_vec();
                    for otr in room_info.objects_to_remove {
                        layers[otr.layer as usize].objects.as_mut_vec()
                            .retain(|i| !otr.instance_ids.contains(&i.instance_id));
                    }
                    Ok(())
                });
            }
            
            if config.qol_cutscenes == CutsceneMode::Major && is_elevator(room_info.room_id.to_u32()) {
                patcher.add_scly_patch(
                    (pak_name.as_bytes(), room_info.room_id.to_u32()),
                    move |ps, area| patch_remove_cutscenes(
                        ps, area,
                        vec![],
                        vec![],
                        true,
                    ),
                );
            }

            // Get list of patches specified for this room
            let (pickups, scans, doors) = {
                let mut _pickups = Vec::new();
                let mut _scans = Vec::new();
                let mut _doors = HashMap::<u32, DoorConfig>::new();
                
                let level = config.level_data.get(world.to_json_key());
                if level.is_some() {
                    let room = level.unwrap().rooms.get(room_info.name.trim());
                    if room.is_some() {
                        let room = room.as_ref().unwrap();
                        if room.pickups.is_some() {
                            _pickups = room.pickups.clone().unwrap();
                        }

                        if room.extra_scans.is_some() {
                            _scans = room.extra_scans.clone().unwrap();
                        }

                        if room.doors.is_some() {
                            _doors = room.doors.clone().unwrap();
                        }
                    }
                }
                (_pickups, _scans, _doors)
            };

            // Patch existing item locations
            let mut idx = 0;
            let pickups_config_len = pickups.len();
            for pickup_location in room_info.pickup_locations.iter() {
                let pickup = {
                    if idx >= pickups_config_len {
                        PickupConfig {
                            pickup_type: "Nothing".to_string(), // TODO: Could figure out the vanilla item instead
                            curr_increase: Some(0),
                            max_increase: Some(0),
                            position: None,
                            hudmemo_text: None,
                            scan_text: None,
                            model: None,
                            respawn: None,
                            modal_hudmemo: None,
                        }
                    } else {
                        pickups[idx].clone() // TODO: cloning is suboptimal
                    }
                };
                
                let key = PickupHashKey {
                    level_id: world.mlvl(),
                    room_id: room_info.room_id.to_u32(),
                    pickup_idx: idx as u32,
                };

                let skip_hudmemos = {
                    if config.qol_cosmetic {
                        !(pickup.modal_hudmemo.clone().unwrap_or(false)) // make them modal if the client specified
                    } else {
                        false // leave them as they are in vanilla, modal
                    }
                };

                let hudmemo_delay = {
                    if pickup.modal_hudmemo.clone().unwrap_or(false) {
                        3.0 // manually specified modal hudmemos are 3s
                    } else {
                        0.0 // otherwise, leave unchanged from vanilla
                    }
                };

                // modify pickup, connections, hudmemo etc.
                patcher.add_scly_patch(
                    (pak_name.as_bytes(), room_info.room_id.to_u32()),
                    move |ps, area| modify_pickups_in_mrea(
                            ps,
                            area,
                            &pickup,
                            *pickup_location,
                            game_resources,
                            pickup_hudmemos,
                            pickup_scans,
                            key,
                            skip_hudmemos,
                            hudmemo_delay,
                            config.qol_pickup_scans,
                            extern_models,
                        )
                );

                idx = idx + 1;
            }

            // Patch extra item locations
            while idx < pickups_config_len {
                let pickup = pickups[idx].clone(); // TODO: cloning is suboptimal

                let key = PickupHashKey {
                    level_id: world.mlvl(),
                    room_id: room_info.room_id.to_u32(),
                    pickup_idx: idx as u32,
                };

                let skip_hudmemos = {
                    if config.qol_cosmetic {
                        !(pickup.modal_hudmemo.clone().unwrap_or(false))
                    } else {
                        true
                    }
                };

                patcher.add_scly_patch(
                    (pak_name.as_bytes(), room_info.room_id.to_u32()),
                    move |_ps, area| patch_add_item(
                        _ps,
                        area,
                        &pickup, 
                        game_resources,
                        pickup_hudmemos,
                        pickup_scans,
                        key,
                        skip_hudmemos,
                        extern_models,
                    ),
                );

                idx = idx + 1;
            }

            // Add extra scans (poi)
            idx = 0;
            for scan in scans.iter() {
                let scan = scan.clone();
                let key = PickupHashKey {
                    level_id: world.mlvl(),
                    room_id: room_info.room_id.to_u32(),
                    pickup_idx: idx as u32,
                };

                let (scan_id, strg_id) = extra_scans.get(&key).unwrap();

                patcher.add_scly_patch(
                    (pak_name.as_bytes(), room_info.room_id.to_u32()),
                    move |ps, area| patch_add_poi(ps, area, game_resources, scan_id.clone(), strg_id.clone(), scan.position),
                );

                idx = idx + 1;
            }

            // Edit doors
            for (dock_num, door_config) in doors {
                // Find the corresponding traced info for this dock
                let mut door_location: Option<DoorLocation> = None;
                for dl in room_info.door_locations {
                    if dl.dock_number.is_some() && dl.dock_number.clone().unwrap() == dock_num {
                        door_location = Some(dl.clone());
                        break;
                    }
                }
                if door_location.is_none() {
                    panic!("Could not find dock #{} in '{}'", dock_num, room_info.name);
                }
                let door_location = door_location.unwrap();

                // If specified, patch this door's connection
                if door_config.destination.is_some() {
                    // TODO: special handling is needed if a dock takes you to a room which is already connected to this one at another dock
                    // TODO: special handling is desired if a dock takes you to it's vanilla destination
                    // TOOD: add scan point on this door to tell what the destination door is
                    // TODO: need to trace dock resources for morph ball tunnels (e.g. fiery shores)
                    // TODO: allow destinations outside this pak

                    // Get info about the destination room
                    let destination = door_config.destination.clone().unwrap();
                    let destination_room = SpawnRoomData::from_str(format!("{}:{}", world.to_str(), destination.room_name).as_str());

                    // Patch the current room to lead to the new destination room
                    patcher.add_scly_patch(
                        (pak_name.as_bytes(), room_info.room_id.to_u32()),
                        move |ps, area| patch_modify_dock(ps, area, dock_num, destination_room.mrea_idx),
                    );

                    // Patch the destination room to "catch" the player with a teleporter at the same location as this room's dock
                    patcher.add_scly_patch(
                        (pak_name.as_bytes(), destination_room.mrea),
                        move |ps, area| patch_add_dock_teleport(
                            ps, area,
                            door_location.dock_position.clone().unwrap(),
                            door_location.dock_scale.clone().unwrap(),
                            destination.dock_num, // Used to determined where the player is placed in the new room
                        ),
                    );
                }
            }
        }
    }

    let (skip_frigate, skip_ending_cinematic) = make_elevators_patch(
        &mut patcher,
        &config.level_data,
        config.auto_enabled_elevators,
        config.ctwk_config.player_size.clone().unwrap_or(1.0),
        config.force_vanilla_layout,
    );

    patcher.add_file_patch(
        b"default.dol",
        |file| patch_dol(
            file,
            starting_room,
            version,
            config,
            remove_ball_color,
        )
    );

    let rel_config = create_rel_config_file(starting_room, config.quickplay);

    if skip_frigate && starting_room.mlvl != World::FrigateOrpheon.mlvl(){
        // remove frigate data to save time/space
        patcher.add_file_patch(b"Metroid1.pak", empty_frigate_pak);
    } else {
        // redirect end of frigate cutscene to room specified in layout
        patcher.add_scly_patch(
            resource_info!("01_intro_hanger.MREA").into(),
            move |_ps, area| patch_teleporter_destination(area, frigate_done_room),
        );

        if move_item_loss_scan {
            patcher.add_scly_patch(
                resource_info!("02_intro_elevator.MREA").into(),
                patch_move_item_loss_scan,
            );
        }
    }

    if essence_done_room.is_some() {
        // redirect end of crater cutscene to room specified in layout
        patcher.add_scly_patch(
            resource_info!("03f_crater.MREA").into(),
            move |_ps, area| patch_teleporter_destination(area, essence_done_room.unwrap()),
        );
    }

    gc_disc.add_file(
        "rel_config.bin",
        structs::FstEntryFile::ExternalFile(Box::new(rel_config)),
    )?;

    if !config.force_vanilla_layout {
        const ARTIFACT_TOTEM_SCAN_STRGS: &[ResourceInfo] = &[
            resource_info!("07_Over_Stonehenge Totem 5.STRG"), // Lifegiver
            resource_info!("07_Over_Stonehenge Totem 4.STRG"), // Wild
            resource_info!("07_Over_Stonehenge Totem 10.STRG"), // World
            resource_info!("07_Over_Stonehenge Totem 9.STRG"), // Sun
            resource_info!("07_Over_Stonehenge Totem 3.STRG"), // Elder
            resource_info!("07_Over_Stonehenge Totem 11.STRG"), // Spirit
            resource_info!("07_Over_Stonehenge Totem 1.STRG"), // Truth
            resource_info!("07_Over_Stonehenge Totem 7.STRG"), // Chozo
            resource_info!("07_Over_Stonehenge Totem 6.STRG"), // Warrior
            resource_info!("07_Over_Stonehenge Totem 12.STRG"), // Newborn
            resource_info!("07_Over_Stonehenge Totem 8.STRG"), // Nature
            resource_info!("07_Over_Stonehenge Totem 2.STRG"), // Strength
        ];
        for (res_info, strg_text) in ARTIFACT_TOTEM_SCAN_STRGS.iter().zip(artifact_totem_strings.iter()) {
            patcher.add_resource_patch(
                (*res_info).into(),
                move |res| patch_artifact_totem_scan_strg(res, &strg_text),
            );
        }
    }

    patcher.add_resource_patch(
        resource_info!("STRG_Main.STRG").into(),// 0x0552a456
        |res| patch_main_strg(res, &config.main_menu_message)
    );
    patcher.add_resource_patch(
        resource_info!("FRME_NewFileSelect.FRME").into(),
        patch_main_menu
    );
    patcher.add_resource_patch(
        resource_info!("STRG_Credits.STRG").into(),
        |res| patch_credits(res, config)
    );
    patcher.add_scly_patch(
        resource_info!("07_stonehenge.MREA").into(),
        |ps, area| fix_artifact_of_truth_requirements(ps, area, config)
    );
    patcher.add_scly_patch(
        resource_info!("07_stonehenge.MREA").into(),
        |ps, area| patch_artifact_hint_availability(ps, area, config.artifact_hint_behavior)
    );

    patcher.add_resource_patch(
        resource_info!("TXTR_SaveBanner.TXTR").into(),
        patch_save_banner_txtr
    );

    if config.patch_power_conduits {
        patch_power_conduits(&mut patcher);
    }

    if config.remove_mine_security_station_locks {
        patcher.add_scly_patch(
            resource_info!("02_mines_shotemup.MREA").into(), // Mines Security Station
            remove_door_locks,
        );
    }

    // TODO: only patch what we need
    patcher.add_resource_patch(
        resource_info!("!TalonOverworld_Master.SAVW").into(),
        move |res| patch_add_scans_to_savw(
            res,
            &savw_scans_to_add,
        ),
    );

    patcher.add_resource_patch(
        resource_info!("!RuinsWorld_Master.SAVW").into(),
        move |res| patch_add_scans_to_savw(
            res,
            &savw_scans_to_add,
        ),
    );

    patcher.add_resource_patch(
        resource_info!("!LavaWorld_Master.SAVW").into(),
        move |res| patch_add_scans_to_savw(
            res,
            &savw_scans_to_add,
        ),
    );

    patcher.add_resource_patch(
        resource_info!("!IceWorld_Master.SAVW").into(),
        move |res| patch_add_scans_to_savw(
            res,
            &savw_scans_to_add,
        ),
    );

    patcher.add_resource_patch(
        resource_info!("!MinesWorld_Master.SAVW").into(),
        move |res| patch_add_scans_to_savw(
            res,
            &savw_scans_to_add,
        ),
    );

    patcher.add_resource_patch(
        resource_info!("!CraterWorld_Master.SAVW").into(),
        move |res| patch_add_scans_to_savw(
            res,
            &savw_scans_to_add,
        ),
    );

    patcher.add_scly_patch(
        (starting_room.pak_name.as_bytes(), starting_room.mrea),
        move |_ps, area| patch_starting_pickups(
            area,
            &config.starting_items,
            show_starting_memo,
            &game_resources,
        )
    );

    if !skip_frigate {
        patcher.add_scly_patch(
            resource_info!("02_intro_elevator.MREA").into(),
            move |_ps, area| patch_starting_pickups(
                area,
                &config.item_loss_items,
                false,
                &game_resources,
            )
        );

        patcher.add_resource_patch(
            resource_info!("!Intro_Master.SAVW").into(),
            move |res| patch_add_scans_to_savw(
                res,
                &savw_scans_to_add,
            ),
        );

        // TODO: only works for landing site
        patcher.add_scly_patch(
            (frigate_done_room.pak_name.as_bytes(), frigate_done_room.mrea),
            move |_ps, area| patch_starting_pickups(
                area,
                &config.item_loss_items,
                false,
                &game_resources,
            )
        );
    }

    if !config.force_vanilla_layout {
        if starting_room.mrea != SpawnRoom::LandingSite.spawn_room_data().mrea || config.qol_cutscenes == CutsceneMode::Major {
            // If we have a non-default start point, patch the landing site to avoid
            // weirdness with cutscene triggers and the ship spawning.
            patcher.add_scly_patch(
                resource_info!("01_over_mainplaza.MREA").into(),
                patch_landing_site_cutscene_triggers
            );
        }
    }

    patch_heat_damage_per_sec(&mut patcher, config.heat_damage_per_sec);
    
    // Always patch out the white flash for photosensitive epileptics
    if version == Version::NtscU0_00 {
        patcher.add_scly_patch(
            resource_info!("03f_crater.MREA").into(),
            patch_essence_cinematic_skip_whitescreen
        );
    }
    if [Version::NtscU0_00, Version::NtscU0_02, Version::Pal].contains(&version) {
        patcher.add_scly_patch(
            resource_info!("03f_crater.MREA").into(),
            patch_essence_cinematic_skip_nomusic
        );
    }

    if config.qol_game_breaking {
        patch_qol_game_breaking(&mut patcher, version, config.force_vanilla_layout, config.ctwk_config.player_size.clone().unwrap_or(1.0) < 0.9);
    }

    // not only is this game-breaking, but it's nonsensical and counterintuitive, always fix //
    patcher.add_scly_patch(
        resource_info!("00i_mines_connect.MREA").into(), // Dynamo Access (Mines)
        move |ps, area| patch_spawn_point_position(ps, area, [0.0, 0.0, 0.0], true, false)
    );

    if config.qol_cosmetic {
        patch_qol_cosmetic(&mut patcher, skip_ending_cinematic);

        // Replace the FMVs that play when you select a file so each ISO always plays the only one.
        const SELECT_GAMES_FMVS: &[&[u8]] = &[
            b"Video/02_start_fileselect_A.thp",
            b"Video/02_start_fileselect_B.thp",
            b"Video/02_start_fileselect_C.thp",
            b"Video/04_fileselect_playgame_A.thp",
            b"Video/04_fileselect_playgame_B.thp",
            b"Video/04_fileselect_playgame_C.thp",
        ];
        for fmv_name in SELECT_GAMES_FMVS {
            let fmv_ref = if fmv_name[7] == b'2' {
                &start_file_select_fmv
            } else {
                &file_select_play_game_fmv
            };
            patcher.add_file_patch(fmv_name, move |file| {
                *file = fmv_ref.clone();
                Ok(())
            });
        }
    }

    if !config.force_vanilla_layout {
        patch_qol_logical(&mut patcher, config);
    }

    if let Some(angle) = config.suit_hue_rotate_angle {
        let iter = PHAZON_SUIT_TEXTURES.iter()
            .chain(VARIA_SUIT_TEXTURES.iter())
            .chain(crate::txtr_conversions::POWER_SUIT_TEXTURES.iter())
            .chain(crate::txtr_conversions::GRAVITY_SUIT_TEXTURES.iter());
        for varia_texture in iter {
            patcher.add_resource_patch((*varia_texture).into(), move |res| {
                let res_data; 
                let data; 
                let mut txtr: structs::Txtr = match &res.kind {
                    structs::ResourceKind::Unknown(_, _) => {
                        res_data = crate::ResourceData::new(res);
                        data = res_data.decompress().into_owned();
                        let mut reader = Reader::new(&data[..]);
                        reader.read(())
                    }, 
                    structs::ResourceKind::External(_, _) => {
                        res_data = crate::ResourceData::new_external(res);
                        data = res_data.decompress().into_owned();
                        let mut reader = Reader::new(&data[..]);
                        reader.read(())
                    }, 
                    _ => panic!("Unsupported resource kind for recoloring."), 
                };

                let mut w = txtr.width as usize;
                let mut h = txtr.height as usize;
                for mipmap in txtr.pixel_data.as_mut_vec() {
                    let mut decompressed_bytes = vec![0u8; w * h * 4];
                    cmpr_decompress(&mipmap.as_mut_vec()[..], h, w, &mut decompressed_bytes[..]);
                    huerotate_in_place(&mut decompressed_bytes[..], w, h, angle);
                    cmpr_compress(&decompressed_bytes[..], w, h, &mut mipmap.as_mut_vec()[..]);
                    w /= 2;
                    h /= 2;
                }
                let mut bytes = vec![];
                txtr.write_to(&mut bytes).unwrap();
                res.kind = structs::ResourceKind::External(bytes, b"TXTR".into());
                res.compressed = false;
                Ok(())
            })
        }
    }
    
    if config.warp_to_start
    {
        const SAVE_STATIONS_ROOMS: &[ResourceInfo] = &[
            // Space Pirate Frigate
            resource_info!("06_intro_to_reactor.MREA"),
            // Chozo Ruins
            resource_info!("1_savestation.MREA"),
            resource_info!("2_savestation.MREA"),
            resource_info!("3_savestation.MREA"),
            // Phendrana Drifts
            resource_info!("mapstation_ice.MREA"),
            resource_info!("savestation_ice_b.MREA"),
            resource_info!("savestation_ice_c.MREA"),
            resource_info!("pickup01.MREA"),
            // Tallon Overworld
            resource_info!("01_over_mainplaza.MREA"),
            resource_info!("06_under_intro_save.MREA"),
            // Phazon Mines
            resource_info!("savestation_mines_a.MREA"),
            resource_info!("00_mines_savestation_c.MREA"),
            resource_info!("00_mines_savestation_d.MREA"),
            // Magmoor Caverns
            resource_info!("lava_savestation_a.MREA"),
            resource_info!("lava_savestation_b.MREA"),
            // Impact Crater
            resource_info!("00_crater_over_elev_j.MREA"),
        ];

        for save_station_room in SAVE_STATIONS_ROOMS.iter() {
            patcher.add_scly_patch(
                (*save_station_room).into(),
                move |ps, area| patch_save_station_for_warp_to_start(
                    ps,
                    area,
                    &game_resources,
                    starting_room,
                    version,
                    config.warp_to_start_delay_s,
                )
            );
        }

        patcher.add_resource_patch(
            resource_info!("STRG_MemoryCard.STRG").into(),// 0x19C3F7F7
            |res| patch_memorycard_strg(res)
        );
    }

    patcher.run(gc_disc)?;
    Ok(())
}
