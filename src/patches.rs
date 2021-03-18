
use encoding::{
    all::WINDOWS_1252,
    Encoding,
    EncoderTrap,
};
use enum_map::EnumMap;
use rand::{
    rngs::StdRng,
    seq::SliceRandom,
    SeedableRng,
    Rng,
};
use serde::Deserialize;

use crate::{
    custom_assets::custom_asset_ids,
    dol_patcher::DolPatcher,
    ciso_writer::CisoWriter,
    elevators::{Elevator, SpawnRoom},
    gcz_writer::GczWriter,
    memmap,
    mlvl_wrapper,
    pickup_meta::{self, PickupType},
    patcher::{PatcherState, PrimePatcher},
    starting_items::StartingItems,
    txtr_conversions::{
        cmpr_compress, cmpr_decompress, huerotate_in_place, VARIA_SUIT_TEXTURES,
        PHAZON_SUIT_TEXTURES,
    },
    GcDiscLookupExtensions,
};

use dol_symbol_table::mp1_symbol;
use resource_info_table::{resource_info, ResourceInfo};
use ppcasm::ppcasm;

use reader_writer::{
    generic_array::GenericArray,
    typenum::U3,
    CStrConversionExtension,
    FourCC,
    LCow,
    Reader,
    Writable,
};
use structs::{res_id, ResId};

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    convert::TryInto,
    ffi::CString,
    fmt,
    fs::File,
    io::Write,
    iter,
    mem,
};

const ARTIFACT_OF_TRUTH_REQ_LAYER: u32 = 24;
const ALWAYS_MODAL_HUDMENUS: &[usize] = &[23, 50, 63];


// When changing a pickup, we need to give the room a copy of the resources/
// assests used by the pickup. Create a cache of all the resources needed by
// any pickup.
fn collect_pickup_resources<'r>(gc_disc: &structs::GcDisc<'r>, starting_items: &StartingItems)
    -> HashMap<(u32, FourCC), structs::Resource<'r>>
{

    let mut looking_for: HashSet<_> = PickupType::iter()
        .flat_map(|pt| pt.dependencies().iter().cloned())
        .chain(PickupType::iter().map(|pt| pt.hudmemo_strg().into()))
        .collect();

    let mut found = HashMap::with_capacity(looking_for.len());

    for pak_name in pickup_meta::ROOM_INFO.iter().map(|(name, _)| name) {
        let file_entry = gc_disc.find_file(pak_name).unwrap();
        let pak = match *file_entry.file().unwrap() {
            structs::FstEntryFile::Pak(ref pak) => Cow::Borrowed(pak),
            structs::FstEntryFile::Unknown(ref reader) => Cow::Owned(reader.clone().read(())),
            _ => panic!(),
        };

        for res in pak.resources.iter() {
            let key = (res.file_id, res.fourcc());
            if looking_for.remove(&key) {
                assert!(found.insert(key, res.into_owned()).is_none());
            }
        }
    }

    for res in crate::custom_assets::custom_assets(&found, starting_items) {
        let key = (res.file_id, res.fourcc());
        looking_for.remove(&key);
        assert!(found.insert(key, res).is_none());
    }

    assert!(looking_for.is_empty());

    found
}

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

fn build_artifact_temple_totem_scan_strings<R>(pickup_layout: &[PickupType], rng: &mut R)
    -> [String; 12]
    where R: Rng
{
    let mut generic_text_templates = [
        "I mean, maybe it'll be in the &push;&main-color=#43CD80;{room}&pop;. I forgot, to be honest.\0",
        "I'm not sure where the artifact exactly is, but like, you can try the &push;&main-color=#43CD80;{room}&pop;.\0",
        "Hey man, so some of the Chozo dudes are telling me that they're might be a thing in the &push;&main-color=#43CD80;{room}&pop;. Just sayin'.\0",
        "Uhh umm... Where was it...? Uhhh, errr, it's definitely in the &push;&main-color=#43CD80;{room}&pop;! I am 100% not totally making it up...\0",
        "Some say it may be in the &push;&main-color=#43CD80;{room}&pop;. Others say that you have no business here. Please leave me alone.\0",
        "So a buddy of mine and I were drinking one night and we thought 'Hey, wouldn't be crazy if we put it at the &push;&main-color=#43CD80;{room}&pop;?' So we did and it took both of us just to get it there!\0",
        "So, uhhh, I kind of got a little lazy and I might have just dropped mine somewhere... Maybe it's in the &push;&main-color=#43CD80;{room}&pop;? Who knows.\0",
        "I uhhh... was a little late to the party and someone had to run out and hide both mine and hers. I owe her one. She told me it might be in the &push;&main-color=#43CD80;{room}&pop;, so you're going to have to trust her on this one.\0",
        "Okay, so this jerk forgets to hide his and I had to hide it for him too. So, I just tossed his somewhere and made up a name for the room. This is literally saving the planet - how can anyone forget that? Anyway, mine is in the &push;&main-color=#43CD80;{room}&pop;, so go check it out. I'm never doing this again...\0",
        "To be honest, I don't know if it was a Missile Expansion or not. Maybe it was... We'll just go with that: There's a Missile Expansion at the &push;&main-color=#43CD80;{room}&pop;.\0",
        "Hear the words of Oh Leer, last Chozo of the Artifact Temple. May they serve you well, that you may find a key lost to our cause... Alright, whatever. It's at the &push;&main-color=#43CD80;{room}&pop;.\0",
        "I kind of just played Frisbee with mine. It flew and landed too far so I didn't want to walk over and grab it because I was lazy. It's in the &push;&main-color=#43CD80;{room}&pop; if you want to find it.\0",
    ];
    generic_text_templates.shuffle(rng);
    let mut generic_templates_iter = generic_text_templates.iter();

    // TODO: If there end up being a large number of these, we could use a binary search
    //       instead of searching linearly.
    // XXX It would be nice if we didn't have to use Vec here and could allocated on the stack
    //     instead, but there doesn't seem to be a way to do it that isn't extremely painful or
    //     relies on unsafe code.
    let mut specific_room_templates = [
        // Artifact Temple
        (0x2398E906, vec!["{pickup} awaits those who truly seek it.\0"]),
    ];
    for rt in &mut specific_room_templates {
        rt.1.shuffle(rng);
    }


    let mut scan_text = [
        String::new(), String::new(), String::new(), String::new(),
        String::new(), String::new(), String::new(), String::new(),
        String::new(), String::new(), String::new(), String::new(),
    ];

    let names_iter = pickup_meta::ROOM_INFO.iter()
        .flat_map(|i| i.1.iter()) // Flatten out the rooms of the paks
        .flat_map(|l| iter::repeat((l.room_id, l.name)).take(l.pickup_locations.len()));
    let iter = pickup_layout.iter()
        .zip(names_iter)
        // ▼▼▼▼ Only yield artifacts ▼▼▼▼
        .filter(|&(pt, _)| pt.is_artifact());

    // Shame there isn't a way to flatten tuples automatically
    for (pt, (room_id, name)) in iter {
        let artifact_id = pt.idx() - PickupType::ArtifactOfLifegiver.idx();
        if scan_text[artifact_id].len() != 0 {
            // If there are multiple of this particular artifact, then we use the first instance
            // for the location of the artifact.
            continue;
        }

        // If there are specific messages for this room, choose one, other wise choose a generic
        // message.
        let template = specific_room_templates.iter_mut()
            .find(|row| row.0 == room_id.to_u32())
            .and_then(|row| row.1.pop())
            .unwrap_or_else(|| generic_templates_iter.next().unwrap());
        let pickup_name = pt.name();
        scan_text[artifact_id] = template.replace("{room}", name).replace("{pickup}", pickup_name);
    }

    // Set a default value for any artifacts that we didn't find.
    for i in 0..scan_text.len() {
        if scan_text[i].len() == 0 {
            scan_text[i] = "Artifact not present. This layout may not be completable.\0".to_owned();
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

fn patch_mines_savw_for_phazon_suit_scan(res: &mut structs::Resource)
    -> Result<(), String>
{
    // Add a scan for the Phazon suit.
    let savw = res.kind.as_savw_mut().unwrap();
    savw.scan_array.as_mut_vec().push(structs::ScannableObject {
        scan: custom_asset_ids::PHAZON_SUIT_SCAN.into(),
        logbook_category: 0,
    });
    Ok(())
}

#[derive(Copy, Clone, Debug)]
enum MaybeObfuscatedPickup
{
    Unobfuscated(PickupType),
    Obfuscated(PickupType),
}

impl MaybeObfuscatedPickup
{
    fn orig(&self) -> PickupType
    {
        match self {
            MaybeObfuscatedPickup::Unobfuscated(pt) => *pt,
            MaybeObfuscatedPickup::Obfuscated(pt) => *pt,
        }
    }

    // fn name(&self) -> &'static str
    // {
    //     self.orig().name()
    // }

    fn dependencies(&self) -> &'static [(u32, FourCC)]
    {
        match self {
            MaybeObfuscatedPickup::Unobfuscated(pt) => pt.dependencies(),
            MaybeObfuscatedPickup::Obfuscated(_) => PickupType::Nothing.dependencies(),
        }
    }

    fn hudmemo_strg(&self) -> ResId<res_id::STRG>
    {
        self.orig().hudmemo_strg()
    }

    fn skip_hudmemos_strg(&self) -> ResId<res_id::STRG>
    {
        self.orig().skip_hudmemos_strg()
    }

    pub fn attainment_audio_file_name(&self) -> &'static str
    {
        self.orig().attainment_audio_file_name()
    }

    pub fn pickup_data<'a>(&self) -> LCow<'a, structs::Pickup<'static>>
    {
        match self {
            MaybeObfuscatedPickup::Unobfuscated(pt) => LCow::Borrowed(pt.pickup_data()),
            MaybeObfuscatedPickup::Obfuscated(original) => {
                let original = original.pickup_data();
                let nothing = PickupType::Nothing.pickup_data();

                LCow::Owned(structs::Pickup {
                    name: original.name.clone(),
                    kind: original.kind,
                    max_increase: original.max_increase,
                    curr_increase: original.curr_increase,
                    ..nothing.clone()
                })
            },
        }
    }
}

fn modify_pickups_in_mrea<'r>(
    ps: &mut PatcherState,
    area: &mut mlvl_wrapper::MlvlArea<'r, '_, '_, '_>,
    pickup_type: PickupType,
    pickup_location: pickup_meta::PickupLocation,
    pickup_resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
    config: &ParsedConfig,
) -> Result<(), String>
{
    let location_idx = 0;

    let pickup_type = if config.obfuscate_items {
        MaybeObfuscatedPickup::Obfuscated(pickup_type)
    } else {
        MaybeObfuscatedPickup::Unobfuscated(pickup_type)
    };

    let deps_iter = pickup_type.dependencies().iter()
        .map(|&(file_id, fourcc)| structs::Dependency {
                asset_id: file_id,
                asset_type: fourcc,
            });

    let name = CString::new(format!(
            "Randomizer - Pickup {} ({:?})", location_idx, pickup_type.pickup_data().name)).unwrap();
    area.add_layer(Cow::Owned(name));

    let new_layer_idx = area.layer_flags.layer_count as usize - 1;

    // Add our custom STRG
    let hudmemo_dep = if config.skip_hudmenus && !ALWAYS_MODAL_HUDMENUS.contains(&location_idx) {
        pickup_type.skip_hudmemos_strg().into()
    } else {
        pickup_type.hudmemo_strg().into()
    };
    let deps_iter = deps_iter.chain(iter::once(hudmemo_dep));
    area.add_dependencies(pickup_resources, new_layer_idx, deps_iter);

    let scly = area.mrea().scly_section_mut();
    let layers = scly.layers.as_mut_vec();

    let mut additional_connections = Vec::new();

    // Add a post-pickup relay. This is used to support cutscene-skipping
    let instance_id = ps.fresh_instance_id_range.next().unwrap();
    let relay = post_pickup_relay_template(instance_id,
                                            pickup_location.post_pickup_relay_connections);
    layers[new_layer_idx].objects.as_mut_vec().push(relay);
    additional_connections.push(structs::Connection {
        state: structs::ConnectionState::ARRIVED,
        message: structs::ConnectionMsg::SET_TO_ZERO,
        target_object_id: instance_id,
    });

    // If this is an artifact, insert a layer change function
    let pickup_kind = pickup_type.pickup_data().kind;
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

    let pickup = layers[pickup_location.location.layer as usize].objects.iter_mut()
        .find(|obj| obj.instance_id ==  pickup_location.location.instance_id)
        .unwrap();
    update_pickup(pickup, pickup_type);
    if additional_connections.len() > 0 {
        pickup.connections.as_mut_vec().extend_from_slice(&additional_connections);
    }

    let hudmemo = layers[pickup_location.hudmemo.layer as usize].objects.iter_mut()
        .find(|obj| obj.instance_id ==  pickup_location.hudmemo.instance_id)
        .unwrap();
    update_hudmemo(hudmemo, pickup_type, location_idx, config.skip_hudmenus);


    let location = pickup_location.attainment_audio;
    let attainment_audio = layers[location.layer as usize].objects.iter_mut()
        .find(|obj| obj.instance_id ==  location.instance_id)
        .unwrap();
    update_attainment_audio(attainment_audio, pickup_type);
    Ok(())
}

fn update_pickup(pickup: &mut structs::SclyObject, pickup_type: MaybeObfuscatedPickup)
{
    let pickup = pickup.property_data.as_pickup_mut().unwrap();
    let original_pickup = pickup.clone();

    let original_aabb = pickup_meta::aabb_for_pickup_cmdl(original_pickup.cmdl).unwrap();
    let new_aabb = pickup_meta::aabb_for_pickup_cmdl(pickup_type.pickup_data().cmdl).unwrap();
    let original_center = calculate_center(original_aabb, original_pickup.rotation,
                                            original_pickup.scale);
    let new_center = calculate_center(new_aabb, pickup_type.pickup_data().rotation,
                                        pickup_type.pickup_data().scale);

    // The pickup needs to be repositioned so that the center of its model
    // matches the center of the original.
    *pickup = structs::Pickup {
        position: [
            original_pickup.position[0] - (new_center[0] - original_center[0]),
            original_pickup.position[1] - (new_center[1] - original_center[1]),
            original_pickup.position[2] - (new_center[2] - original_center[2]),
        ].into(),
        hitbox: original_pickup.hitbox,
        scan_offset: [
            original_pickup.scan_offset[0] + (new_center[0] - original_center[0]),
            original_pickup.scan_offset[1] + (new_center[1] - original_center[1]),
            original_pickup.scan_offset[2] + (new_center[2] - original_center[2]),
        ].into(),

        fade_in_timer: original_pickup.fade_in_timer,
        spawn_delay: original_pickup.spawn_delay,
        active: original_pickup.active,

        ..(pickup_type.pickup_data().into_owned())
    };
}

fn update_hudmemo(
    hudmemo: &mut structs::SclyObject,
    pickup_type: MaybeObfuscatedPickup,
    location_idx: usize,
    skip_hudmenus: bool)
{
    // The items in Watery Hall (Charge beam), Research Core (Thermal Visor), and Artifact Temple
    // (Artifact of Truth) should always have modal hudmenus because a cutscene plays immediately
    // after each item is acquired, and the nonmodal hudmenu wouldn't properly appear.
    let hudmemo = hudmemo.property_data.as_hud_memo_mut().unwrap();
    if skip_hudmenus && !ALWAYS_MODAL_HUDMENUS.contains(&location_idx) {
        hudmemo.first_message_timer = 5.;
        hudmemo.memo_type = 0;
        hudmemo.strg = pickup_type.skip_hudmemos_strg();
    } else {
        hudmemo.strg = pickup_type.hudmemo_strg();
    }
}

fn update_attainment_audio(attainment_audio: &mut structs::SclyObject,
                           pickup_type: MaybeObfuscatedPickup)
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


fn make_elevators_patch<'a>(
    patcher: &mut PrimePatcher<'_, 'a>,
    layout: &'a EnumMap<Elevator, SpawnRoom>,
    auto_enabled_elevators: bool,
)
{
    for (elv, dest) in layout.iter() {
        patcher.add_scly_patch((elv.pak_name.as_bytes(), elv.mrea), move |ps, area| {
            let scly = area.mrea().scly_section_mut();
            for layer in scly.layers.iter_mut() {
                let obj = layer.objects.iter_mut()
                    .find(|obj| obj.instance_id == elv.scly_id);
                if let Some(obj) = obj {
                    let wt = obj.property_data.as_world_transporter_mut().unwrap();
                    wt.mrea = ResId::new(dest.mrea);
                    wt.mlvl = ResId::new(dest.mlvl);
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

        let room_dest_name = dest.name.replace('\0', "\n");
        let hologram_name = dest.name.replace('\0', " ");
        let control_name = dest.name.replace('\0', " ");
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


fn patch_frigate_teleporter<'r>(area: &mut mlvl_wrapper::MlvlArea, spawn_room: SpawnRoom)
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
    pickup_layout: &[PickupType],
) -> Result<(), String>
{
    let truth_req_layer_id = area.layer_flags.layer_count;
    assert_eq!(truth_req_layer_id, ARTIFACT_OF_TRUTH_REQ_LAYER);

    // Create a new layer that will be toggled on when the Artifact of Truth is collected
    area.add_layer(b"Randomizer - Got Artifact 1\0".as_cstr());

    let at_pickup_kind = pickup_layout[63].pickup_data().kind;
    for i in 0..12 {
        let layer_number = if i == 0 {
            truth_req_layer_id
        } else {
            i + 1
        };
        let kind = i + 29;
        let exists = pickup_layout.iter()
            .any(|pt| kind == pt.pickup_data().kind);
        if exists && at_pickup_kind != kind {
            // If the artifact exsts, but is not the artifact at the Artifact Temple, mark this
            // layer as inactive. It will be activated when the item is collected.
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

fn patch_temple_security_station_cutscene_trigger(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let trigger = scly.layers.iter_mut()
        .flat_map(|layer| layer.objects.iter_mut())
        .find(|obj| obj.instance_id == 0x70067)
        .and_then(|obj| obj.property_data.as_trigger_mut())
        .unwrap();
    trigger.active = 0;

    Ok(())
}

fn patch_ridley_phendrana_shorelines_cinematic(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    scly.layers.as_mut_vec()[4].objects.as_mut_vec().clear();
    Ok(())
}

fn patch_mqa_cinematic(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    let flags = &mut area.layer_flags.flags;
        *flags &= !(1 << 4); // Turn off the "Room unveil cinematic"

    let mut next_object_id = 0;
    let scly = area.mrea().scly_section_mut();

    for obj in scly.layers.as_mut_vec()[0].objects.iter_mut() {
        if next_object_id < obj.instance_id {
            next_object_id = obj.instance_id;
        }
    }

    let camera_door_id = 0x2000CF;
    let memory_relay_id = 0x2006DE;
    let timer_activate_memory_relay_id = next_object_id + 1;

    scly.layers.as_mut_vec()[0].objects.as_mut_vec().push(
        structs::SclyObject {
            instance_id: timer_activate_memory_relay_id,
            property_data: structs::Timer {
                name: b"Timer - Activate post cutscene memory relay\0".as_cstr(),

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
                    target_object_id: memory_relay_id,
                },
            ].into(),
        }
    );

    let memory_relay_obj = scly.layers.as_mut_vec()[0].objects.as_mut_vec().iter_mut()
        .find(|obj| obj.instance_id == memory_relay_id)
        .unwrap();
    memory_relay_obj.connections.as_mut_vec().push(structs::Connection {
            state: structs::ConnectionState::ACTIVE,
            message: structs::ConnectionMsg::DEACTIVATE,
            target_object_id: timer_activate_memory_relay_id,
        });

    scly.layers.as_mut_vec()[0].objects.as_mut_vec().retain(|obj| obj.instance_id != camera_door_id);
    scly.layers.as_mut_vec()[4].objects.as_mut_vec().clear();

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
    let layer = &mut scly.layers.as_mut_vec()[0];

    // Disable the the trigger when all the pirates are killed
    let obj = layer.objects.as_mut_vec().iter_mut()
        .find(|obj| obj.instance_id == 460074)
        .unwrap();
    obj.connections.as_mut_vec().push(structs::Connection {
            state: structs::ConnectionState::MAX_REACHED,
            message: structs::ConnectionMsg::DEACTIVATE,
            target_object_id: 67568447,
        });
    // TODO: Trigger a MemoryRelay too

    // TODO: Instead of the above, when you pass through a trigger near the "other" door, disable
    // the all of triggers related to the cutscenes in the room.
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


fn patch_credits(res: &mut structs::Resource, pickup_layout: &[PickupType])
    -> Result<(), String>
{
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

    let mut output = concat!(
        "\n\n\n\n\n\n\n",
        "&push;&font=C29C51F1;&main-color=#89D6FF;",
        "Major Item Locations",
        "&pop;",
    ).to_owned();
    for pickup_type in PICKUPS_TO_PRINT {
        let room_idx = if let Some(i) = pickup_layout.iter().position(|i| i == pickup_type) {
            i
        } else {
            continue
        };
        let room_name = pickup_meta::ROOM_INFO.iter()
            .flat_map(|pak_locs| pak_locs.1.iter())
            .flat_map(|loc| iter::repeat(loc.name).take(loc.pickup_locations.len()))
            .nth(room_idx)
            .unwrap();
        let pickup_name = pickup_type.name();
        write!(output, "\n\n{}: {}", pickup_name, room_name).unwrap();
    }
    output += "\n\n\n\n\0";
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
    show_starting_items: bool,
    pickup_resources: &HashMap<(u32, FourCC), structs::Resource<'r>>,
) -> Result<(), String>
{
    let room_id = area.mlvl_area.internal_id;
    let layer_count = area.mrea().scly_section_mut().layers.as_mut_vec().len() as u32;

    if show_starting_items {
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

    if show_starting_items {
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
            &pickup_resources,
            0,
            iter::once(custom_asset_ids::STARTING_ITEMS_HUDMEMO_STRG.into())
        );
    }
    Ok(())
}

include!("../compile_to_ppc/patches_config.rs");
fn create_rel_config_file(
    spawn_room: SpawnRoom,
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
    spawn_room: SpawnRoom,
    version: Version,
    config: &ParsedConfig,
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

    // let ball_color_patch = ppcasm!(symbol_addr!("skBallInnerGlowColors", version), {
    //     .asciiz b"\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff";
    // });
    // dol_patcher.ppcasm_patch(&ball_color_patch)?;
    // let ball_color_patch = ppcasm!(symbol_addr!("BallAuxGlowColors", version), {
    //     .asciiz b"\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff";
    // });
    // dol_patcher.ppcasm_patch(&ball_color_patch)?;
    // let ball_color_patch = ppcasm!(symbol_addr!("BallTransFlashColors", version), {
    //     .asciiz b"\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff";
    // });
    // dol_patcher.ppcasm_patch(&ball_color_patch)?;
    // let ball_color_patch = ppcasm!(symbol_addr!("BallSwooshColors", version), {
    //     .asciiz b"\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff";
    // });
    // dol_patcher.ppcasm_patch(&ball_color_patch)?;
    // let ball_color_patch = ppcasm!(symbol_addr!("BallSwooshColorsJaggy", version), {
    //     .asciiz b"\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff";
    // });
    // dol_patcher.ppcasm_patch(&ball_color_patch)?;
    // let ball_color_patch = ppcasm!(symbol_addr!("BallSwooshColorsCharged", version), {
    //     .asciiz b"\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff";
    // });
    // dol_patcher.ppcasm_patch(&ball_color_patch)?;
    // let ball_color_patch = ppcasm!(symbol_addr!("BallGlowColors", version), {
    //     .asciiz b"\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff";
    // });
    // dol_patcher.ppcasm_patch(&ball_color_patch)?;

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

    if config.max_obtainable_missiles > 999 {
        Err("The max amount of missiles you can carry has exceeded the limit (>999)!".to_string())?;
    }

    if config.max_obtainable_power_bombs > 9 {
        Err("The max amount of power bombs you can carry has exceeded the limit (>9)!".to_string())?;
    }

    // CPlayerState_PowerUpMaxValues[4]
    let max_obtainable_missiles_patch = ppcasm!(symbol_addr!("CPlayerState_PowerUpMaxValues", version) + 0x10, {
        .long config.max_obtainable_missiles;
    });
    dol_patcher.ppcasm_patch(&max_obtainable_missiles_patch)?;

    // CPlayerState_PowerUpMaxValues[7]
    let max_obtainable_power_bombs_patch = ppcasm!(symbol_addr!("CPlayerState_PowerUpMaxValues", version) + 0x1c, {
        .long config.max_obtainable_power_bombs;
    });
    dol_patcher.ppcasm_patch(&max_obtainable_power_bombs_patch)?;

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

fn patch_bnr(file: &mut structs::FstEntryFile, config: &ParsedConfig) -> Result<(), String>
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

    write_encoded_str("game_name", &config.bnr_game_name, &mut bnr.english_fields.game_name)?;
    write_encoded_str("developer", &config.bnr_developer, &mut bnr.english_fields.developer)?;
    write_encoded_str(
        "game_name_full",
        &config.bnr_game_name_full,
        &mut bnr.english_fields.game_name_full
    )?;
    write_encoded_str(
        "developer_full",
        &config.bnr_developer_full,
        &mut bnr.english_fields.developer_full)
    ?;
    write_encoded_str("description", &config.bnr_description, &mut bnr.english_fields.description)?;

    Ok(())
}

// XXX Deserialize is implemented here for c_interface. Ideally this could be done in
//     c_interface.rs itself...
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum IsoFormat
{
    Iso,
    Gcz,
    Ciso,
}

impl Default for IsoFormat
{
    fn default() -> IsoFormat
    {
        IsoFormat::Iso
    }
}

#[derive(Deserialize, Copy, Clone)]
#[serde(rename_all = "camelCase")]
pub enum ArtifactHintBehavior
{
    Default,
    None,
    All,
}

impl Default for ArtifactHintBehavior
{
    fn default() -> Self
    {
        ArtifactHintBehavior::Default
    }
}

pub struct ParsedConfig
{
    pub input_iso: memmap::Mmap,
    pub output_iso: File,
    // pub layout_string: String,

    pub layout: crate::Layout,

    pub iso_format: IsoFormat,
    pub skip_frigate: bool,
    pub skip_hudmenus: bool,
    pub keep_fmvs: bool,
    pub obfuscate_items: bool,
    pub etank_capacity: u32,
    pub nonvaria_heat_damage: bool,
    pub heat_damage_per_sec: f32,
    pub staggered_suit_damage: bool,
    pub max_obtainable_missiles: u32,
    pub max_obtainable_power_bombs: u32,
    pub auto_enabled_elevators: bool,
    pub quiet: bool,

    pub enable_vault_ledge_door: bool,
    pub artifact_hint_behavior: ArtifactHintBehavior,

    pub flaahgra_music_files: Option<[nod_wrapper::FileWrapper; 2]>,

    pub suit_hue_rotate_angle: Option<i32>,

    pub starting_items: StartingItems,
    pub random_starting_items: StartingItems,
    pub comment: String,
    pub main_menu_message: String,

    pub quickplay: bool,

    pub bnr_game_name: Option<String>,
    pub bnr_developer: Option<String>,

    pub bnr_game_name_full: Option<String>,
    pub bnr_developer_full: Option<String>,
    pub bnr_description: Option<String>,
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

pub fn patch_iso<T>(config: ParsedConfig, mut pn: T) -> Result<(), String>
    where T: structs::ProgressNotifier
{
    let mut ct = Vec::new();
    writeln!(ct, "Created by randomprime version {}", env!("CARGO_PKG_VERSION")).unwrap();
    writeln!(ct).unwrap();
    writeln!(ct, "Options used:").unwrap();
    writeln!(ct, "layout: {:#?}", config.layout).unwrap();
    writeln!(ct, "skip frigate: {}", config.skip_frigate).unwrap();
    writeln!(ct, "keep fmvs: {}", config.keep_fmvs).unwrap();
    writeln!(ct, "nonmodal hudmemos: {}", config.skip_hudmenus).unwrap();
    writeln!(ct, "obfuscated items: {}", config.obfuscate_items).unwrap();
    writeln!(ct, "nonvaria heat damage: {}", config.nonvaria_heat_damage).unwrap();
    writeln!(ct, "heat damage per sec: {}", config.heat_damage_per_sec).unwrap();
    writeln!(ct, "staggered suit damage: {}", config.staggered_suit_damage).unwrap();
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
        Version::Pal         => Some(rel_files::PATCHES_PAL_REL),
        Version::NtscJ    => None,
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

fn build_and_run_patches(gc_disc: &mut structs::GcDisc, config: &ParsedConfig, version: Version)
    -> Result<(), String>
{
    let pickup_layout = &config.layout.pickups[..];
    let elevator_layout = &config.layout.elevators;
    let spawn_room = config.layout.starting_location;

    let mut rng = StdRng::seed_from_u64(config.layout.seed);
    let artifact_totem_strings = build_artifact_temple_totem_scan_strings(pickup_layout, &mut rng);

    let pickup_resources = collect_pickup_resources(gc_disc, &config.random_starting_items);
    let starting_items = StartingItems::merge(config.starting_items.clone(), config.random_starting_items.clone());

    // XXX These values need to out live the patcher
    let select_game_fmv_suffix = ["A", "B", "C"].choose(&mut rng).unwrap();
    let n = format!("Video/02_start_fileselect_{}.thp", select_game_fmv_suffix);
    let start_file_select_fmv = gc_disc.find_file(&n).unwrap().file().unwrap().clone();
    let n = format!("Video/04_fileselect_playgame_{}.thp", select_game_fmv_suffix);
    let file_select_play_game_fmv = gc_disc.find_file(&n).unwrap().file().unwrap().clone();


    let pickup_resources = &pickup_resources;
    let mut patcher = PrimePatcher::new();
    patcher.add_file_patch(b"opening.bnr", |file| patch_bnr(file, config));
    if !config.keep_fmvs {
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
    }

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

    // Patch pickups
    let mut layout_iterator = pickup_layout.iter();
    for (name, rooms) in pickup_meta::ROOM_INFO.iter() {
        for room_info in rooms.iter() {
             patcher.add_scly_patch((name.as_bytes(), room_info.room_id.to_u32()), move |_, area| {
                // Remove objects
                let layers = area.mrea().scly_section_mut().layers.as_mut_vec();
                for otr in room_info.objects_to_remove {
                    layers[otr.layer as usize].objects.as_mut_vec()
                        .retain(|i| !otr.instance_ids.contains(&i.instance_id));
                }
                Ok(())
            });
            let iter = room_info.pickup_locations.iter().zip(&mut layout_iterator);
            for (&pickup_location, &pickup_type) in iter {
                // 1 in 1024 chance of a missile being shiny means a player is likely to see a
                // shiny missile every 40ish games (assuming most players collect about half of the
                // missiles)
                let pickup_type = if pickup_type == PickupType::Missile && rng.gen_ratio(1, 1024) {
                    PickupType::ShinyMissile
                } else {
                    pickup_type
                };
                patcher.add_scly_patch(
                    (name.as_bytes(), room_info.room_id.to_u32()),
                    move |ps, area| modify_pickups_in_mrea(
                            ps,
                            area,
                            pickup_type,
                            pickup_location,
                            pickup_resources,
                            config
                        )
                );
            }
        }
    }

    let rel_config;
    if config.skip_frigate {
        patcher.add_file_patch(
            b"default.dol",
            move |file| patch_dol(
                file,
                spawn_room,
                version,
                config,
            )
        );
        patcher.add_file_patch(b"Metroid1.pak", empty_frigate_pak);
        rel_config = create_rel_config_file(spawn_room, config.quickplay);
    } else {
        patcher.add_file_patch(
            b"default.dol",
            |file| patch_dol(
                file,
                SpawnRoom::FrigateExteriorDockingHangar,
                version,
                config,
            )
        );

        patcher.add_scly_patch(
            resource_info!("01_intro_hanger.MREA").into(),
            move |_ps, area| patch_frigate_teleporter(area, spawn_room)
        );
        rel_config = create_rel_config_file(
            SpawnRoom::FrigateExteriorDockingHangar,
            config.quickplay
        );
    }
    gc_disc.add_file(
        "rel_config.bin",
        structs::FstEntryFile::ExternalFile(Box::new(rel_config)),
    )?;

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
        |res| patch_credits(res, &pickup_layout)
    );

    patcher.add_resource_patch(
        resource_info!("!MinesWorld_Master.SAVW").into(),
        patch_mines_savw_for_phazon_suit_scan
    );
    patcher.add_scly_patch(
        resource_info!("07_stonehenge.MREA").into(),
        |ps, area| fix_artifact_of_truth_requirements(ps, area, &pickup_layout)
    );
    patcher.add_scly_patch(
        resource_info!("07_stonehenge.MREA").into(),
        |ps, area| patch_artifact_hint_availability(ps, area, config.artifact_hint_behavior)
    );

    patcher.add_resource_patch(
        resource_info!("TXTR_SaveBanner.TXTR").into(),
        patch_save_banner_txtr
    );

    let show_starting_items = !config.random_starting_items.is_empty();
    patcher.add_scly_patch(
        (spawn_room.pak_name.as_bytes(), spawn_room.mrea),
        move |_ps, area| patch_starting_pickups(
            area,
            &starting_items,
            show_starting_items,
            &pickup_resources,
        )
    );

    patcher.add_resource_patch(resource_info!("FRME_BallHud.FRME").into(), patch_morphball_hud);

    make_elevators_patch(&mut patcher, &elevator_layout, config.auto_enabled_elevators);

    make_elite_research_fight_prereq_patches(&mut patcher);

    patch_heat_damage_per_sec(&mut patcher, config.heat_damage_per_sec);

    patcher.add_scly_patch(
        resource_info!("22_Flaahgra.MREA").into(),
        patch_sunchamber_prevent_wild_before_flaahgra
    );
    patcher.add_scly_patch(
        resource_info!("0v_connect_tunnel.MREA").into(),
        patch_sun_tower_prevent_wild_before_flaahgra
    );
    patcher.add_scly_patch(
        resource_info!("00j_over_hall.MREA").into(),
        patch_temple_security_station_cutscene_trigger
    );
    patcher.add_scly_patch(
        resource_info!("01_ice_plaza.MREA").into(),
        patch_ridley_phendrana_shorelines_cinematic
    );
    patcher.add_scly_patch(
        resource_info!("08_mines.MREA").into(),
        patch_mqa_cinematic
    );
    patcher.add_scly_patch(
        resource_info!("08b_under_intro_ventshaft.MREA").into(),
        patch_main_ventilation_shaft_section_b_door
    );
    patcher.add_scly_patch(
        resource_info!("10_ice_research_a.MREA").into(),
        patch_research_lab_hydra_barrier
    );
    patcher.add_scly_patch(
        resource_info!("12_ice_research_b.MREA").into(),
        move |ps, area| patch_lab_aether_cutscene_trigger(ps, area, version)
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
    patcher.add_scly_patch(
        resource_info!("01_mines_mainplaza.MREA").into(),
        patch_main_quarry_barrier
    );

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

    if spawn_room != SpawnRoom::LandingSite {
        // If we have a non-default start point, patch the landing site to avoid
        // weirdness with cutscene triggers and the ship spawning.
        patcher.add_scly_patch(
            resource_info!("01_over_mainplaza.MREA").into(),
            patch_landing_site_cutscene_triggers
        );
    }

    // If any of the elevators go straight to the ending, patch out the pre-credits cutscene.
    let skip_ending_cinematic = elevator_layout.values()
        .any(|sr| sr == &SpawnRoom::EndingCinematic);
    if skip_ending_cinematic {
        patcher.add_scly_patch(
            resource_info!("01_endcinema.MREA").into(),
            patch_ending_scene_straight_to_credits
        );
    }

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

    if config.enable_vault_ledge_door {
        patcher.add_scly_patch(
            resource_info!("01_mainplaza.MREA").into(),
            make_main_plaza_locked_door_two_ways
        );
    }

    if let Some(angle) = config.suit_hue_rotate_angle {
        let iter = VARIA_SUIT_TEXTURES.iter()
            .chain(PHAZON_SUIT_TEXTURES.iter())
            .chain(crate::txtr_conversions::POWER_SUIT_TEXTURES.iter())
            .chain(crate::txtr_conversions::GRAVITY_SUIT_TEXTURES.iter());
        for varia_texture in iter {
            patcher.add_resource_patch((*varia_texture).into(), move |res| {
                let res_data = crate::ResourceData::new(res);
                let data = res_data.decompress();
                let mut reader = Reader::new(&data[..]);
                let mut txtr: structs::Txtr = reader.read(());

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

    patcher.run(gc_disc)?;
    Ok(())
}

