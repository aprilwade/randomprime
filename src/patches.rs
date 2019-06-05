
use rand::{ChaChaRng, SeedableRng, Rng, Rand};
use encoding::{
    all::WINDOWS_1252,
    Encoding,
    EncoderTrap,
};
use serde_derive::Deserialize;

use crate::{
    asset_ids,
    dol_patcher::DolPatcher,
    ciso_writer::CisoWriter,
    elevators::{ELEVATORS, SpawnRoom},
    gcz_writer::GczWriter,
    memmap,
    mlvl_wrapper,
    pickup_meta::{self, PickupType},
    reader_writer,
    patcher::{PatcherState, PrimePatcher},
    structs,
    GcDiscLookupExtensions,
    ResourceData,
};

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

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    ffi::CString,
    fs::File,
    io::{self, Write},
    iter,
};

const ARTIFACT_OF_TRUTH_REQ_LAYER: u32 = 24;
const ALWAYS_MODAL_HUDMENUS: &[usize] = &[23, 50, 63];


// When changing a pickup, we need to give the room a copy of the resources/
// assests used by the pickup. Create a cache of all the resources needed by
// any pickup.
fn collect_pickup_resources<'r>(gc_disc: &structs::GcDisc<'r>)
    -> HashMap<(u32, FourCC), structs::Resource<'r>>
{
    let mut looking_for: HashSet<_> = PickupType::iter()
        .flat_map(|pt| pt.dependencies().iter().cloned())
        .chain(PickupType::iter().map(|pt| (pt.hudmemo_strg(), b"STRG".into())))
        .collect();

    let mut found = HashMap::with_capacity(looking_for.len());

    let extra_assets = pickup_meta::extra_assets();
    for res in extra_assets {
        looking_for.remove(&(res.file_id, res.fourcc()));
        assert!(found.insert((res.file_id, res.fourcc()), res.clone()).is_none());
    }

    for pak_name in pickup_meta::PICKUP_LOCATIONS.iter().map(|(name, _)| name) {
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

    // Generate and add the assets for Nothing and Phazon Suit
    // XXX This is super gross because arrays don't have owned-iterators
    let new_assets = vec![create_nothing_cmdl_and_ancs(&mut found),
                          create_phazon_cmdl_and_ancs(&mut found)]
                    .into_iter()
                    .flat_map(|(a, b)| vec![a, b].into_iter());
    for res in new_assets {
        let key = (res.file_id, res.fourcc());
        if looking_for.remove(&key) {
            assert!(found.insert(key, res).is_none());
        }
    }

    assert!(looking_for.is_empty());

    found
}

// TODO Reduce duplication between create_phazon_cmdl_and_ancs and create_nothing_cmdl_and_ancs
fn create_nothing_cmdl_and_ancs<'r>(resources: &mut HashMap<(u32, FourCC), structs::Resource<'r>>)
    -> (structs::Resource<'r>, structs::Resource<'r>)
{
    let nothing_suit_cmdl = {
        let grav_suit_cmdl = ResourceData::new(&resources[&(
                asset_ids::GRAVITY_SUIT_CMDL, b"CMDL".into())]);
        let mut nothing_cmdl_bytes = grav_suit_cmdl.decompress().into_owned();

        // Ensure the length is a multiple of 32
        let len = nothing_cmdl_bytes.len();
        nothing_cmdl_bytes.extend(reader_writer::pad_bytes(32, len).iter());

        // Change which texture this points to
        asset_ids::NOTHING_TXTR.write_to(&mut &mut nothing_cmdl_bytes[0x64..]).unwrap();
        asset_ids::PHAZON_SUIT_TXTR2.write_to(&mut &mut nothing_cmdl_bytes[0x70..]).unwrap();
        pickup_meta::build_resource(
            asset_ids::NOTHING_CMDL,
            structs::ResourceKind::External(nothing_cmdl_bytes, b"CMDL".into())
        )
    };
    let nothing_suit_ancs = {
        let grav_suit_ancs = ResourceData::new(&resources[&(
                asset_ids::GRAVITY_SUIT_ANCS, b"ANCS".into())]);
        let mut nothing_ancs_bytes = grav_suit_ancs.decompress().into_owned();

        // Ensure the length is a multiple of 32
        let len = nothing_ancs_bytes.len();
        nothing_ancs_bytes.extend(reader_writer::pad_bytes(32, len).iter());

        // Change this to refer to the CMDL above
        asset_ids::NOTHING_CMDL.write_to(&mut &mut nothing_ancs_bytes[0x14..]).unwrap();
        pickup_meta::build_resource(
            asset_ids::NOTHING_ANCS,
            structs::ResourceKind::External(nothing_ancs_bytes, b"ANCS".into())
        )
    };
    (nothing_suit_cmdl, nothing_suit_ancs)
}

fn create_phazon_cmdl_and_ancs<'r>(resources: &mut HashMap<(u32, FourCC), structs::Resource<'r>>)
    -> (structs::Resource<'r>, structs::Resource<'r>)
{
    let phazon_suit_cmdl = {
        let grav_suit_cmdl = ResourceData::new(&resources[&(
                asset_ids::GRAVITY_SUIT_CMDL, b"CMDL".into())]);
        let mut phazon_cmdl_bytes = grav_suit_cmdl.decompress().into_owned();

        // Ensure the length is a multiple of 32
        let len = phazon_cmdl_bytes.len();
        phazon_cmdl_bytes.extend(reader_writer::pad_bytes(32, len).iter());

        // Change which textures this points to
        asset_ids::PHAZON_SUIT_TXTR1.write_to(&mut &mut phazon_cmdl_bytes[0x64..]).unwrap();
        asset_ids::PHAZON_SUIT_TXTR2.write_to(&mut &mut phazon_cmdl_bytes[0x70..]).unwrap();
        pickup_meta::build_resource(
            asset_ids::PHAZON_SUIT_CMDL,
            structs::ResourceKind::External(phazon_cmdl_bytes, b"CMDL".into())
        )
    };
    let phazon_suit_ancs = {
        let grav_suit_ancs = ResourceData::new(&resources[&(
                asset_ids::GRAVITY_SUIT_ANCS, b"ANCS".into())]);
        let mut phazon_ancs_bytes = grav_suit_ancs.decompress().into_owned();

        // Ensure the length is a multiple of 32
        let len = phazon_ancs_bytes.len();
        phazon_ancs_bytes.extend(reader_writer::pad_bytes(32, len).iter());

        // Change this to refer to the CMDL above
        asset_ids::PHAZON_SUIT_CMDL.write_to(&mut &mut phazon_ancs_bytes[0x14..]).unwrap();
        pickup_meta::build_resource(
            asset_ids::PHAZON_SUIT_ANCS,
            structs::ResourceKind::External(phazon_ancs_bytes, b"ANCS".into())
        )
    };
    (phazon_suit_cmdl, phazon_suit_ancs)
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
        property_data: structs::SclyProperty::SpecialFunction(
            structs::SpecialFunction {
                name: b"Artifact Layer Switch\0".as_cstr(),
                position: [0., 0., 0.].into(),
                rotation: [0., 0., 0.].into(),
                type_: 16,
                unknown0: b"\0".as_cstr(),
                unknown1: 0.,
                unknown2: 0.,
                unknown3: 0.,
                layer_change_room_id: 0xCD2B0EA2,
                layer_change_layer_id: layer,
                item_id: 0,
                unknown4: 1,
                unknown5: 0.,
                unknown6: 0xFFFFFFFF,
                unknown7: 0xFFFFFFFF,
                unknown8: 0xFFFFFFFF,
            }
        ),
    }
}

fn post_pickup_relay_template<'r>(instance_id: u32, connections: &'static [structs::Connection])
    -> structs::SclyObject<'r>
{
    structs::SclyObject {
        instance_id,
        connections: connections.to_owned().into(),
        property_data: structs::SclyProperty::Relay(structs::Relay {
            name: b"Randomizer Post Pickup Relay\0".as_cstr(),
            active: 1,
        })
    }
}

fn add_skip_hudmemos_strgs(pickup_resources: &mut HashMap<(u32, FourCC), structs::Resource>)
{
    for pt in PickupType::iter() {
        let id = pt.skip_hudmemos_strg();
        let res = pickup_meta::build_resource(
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
        );
        assert!(pickup_resources.insert((id, b"STRG".into()), res).is_none())
    }
}

fn build_artifact_temple_totem_scan_strings<R>(pickup_layout: &[PickupType], rng: &mut R)
    -> [String; 12]
    where R: Rng + Rand
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
    rng.shuffle(&mut generic_text_templates);
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
        rng.shuffle(&mut rt.1);
    }


    let mut scan_text = [
        String::new(), String::new(), String::new(), String::new(),
        String::new(), String::new(), String::new(), String::new(),
        String::new(), String::new(), String::new(), String::new(),
    ];

    let names_iter = pickup_meta::PICKUP_LOCATIONS.iter()
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
            .find(|row| row.0 == room_id)
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


fn patch_mines_savw_for_phazon_suit_scan(res: &mut structs::Resource)
    -> Result<(), String>
{
            // Some((asset_ids::PHAZON_MINES_SAVW, fourcc)) if fourcc == b"SAVW".into() => {
    // Add a scan for the Phazon suit.
    let savw = res.kind.as_savw_mut().unwrap();
    savw.scan_array.as_mut_vec().push(structs::ScannableObject {
        scan: asset_ids::PHAZON_SUIT_SCAN,
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

    fn hudmemo_strg(&self) -> u32
    {
        self.orig().hudmemo_strg()
    }

    fn skip_hudmemos_strg(&self) -> u32
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
    let hudmemo_dep = structs::Dependency {
        asset_id: if config.skip_hudmenus && !ALWAYS_MODAL_HUDMENUS.contains(&location_idx) {
                pickup_type.skip_hudmemos_strg()
            } else {
                pickup_type.hudmemo_strg()
            },
        asset_type: b"STRG".into(),
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
    // (Artifact of Truth) should always have modal hudmenus to because a cutscene plays
    // immediately after each item is acquired, and the nonmodal hudmenu wouldn't properly appear.
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


fn make_elevators_patch<'a>(patcher: &mut PrimePatcher<'_, 'a>, layout: &'a [u8])
{
    for (i, elv) in ELEVATORS.iter().enumerate() {
        patcher.add_scly_patch(elv.pak_name.as_bytes(), elv.mrea, move |_ps, area| {
            let scly = area.mrea().scly_section_mut();
            for layer in scly.layers.iter_mut() {
                let obj = layer.objects.iter_mut()
                    .find(|obj| obj.instance_id == elv.scly_id);
                if let Some(obj) = obj {
                    let wt = obj.property_data.as_world_transporter_mut().unwrap();
                    wt.mrea = ELEVATORS[layout[i] as usize].mrea;
                    wt.mlvl = ELEVATORS[layout[i] as usize].mlvl;
                }
            }
            Ok(())
        });

        patcher.add_resource_patch(elv.pak_name.as_bytes(), b"STRG".into(), elv.room_strg, move |res| {
            let string = format!("Transport to {}\u{0}", ELEVATORS[layout[i] as usize].name);
            let strg = structs::Strg::from_strings(vec![string]);
            res.kind = structs::ResourceKind::Strg(strg);
            Ok(())
        });
        patcher.add_resource_patch(elv.pak_name.as_bytes(), b"STRG".into(), elv.hologram_strg, move |res| {
            let string = format!("Access to &main-color=#FF3333;{} &main-color=#89D6FF;granted. Please step into the hologram.\u{0}", ELEVATORS[layout[i] as usize].name);
            let strg = structs::Strg::from_strings(vec![string]);
            res.kind = structs::ResourceKind::Strg(strg);
            Ok(())
        });
        patcher.add_resource_patch(elv.pak_name.as_bytes(), b"STRG".into(), elv.control_strg, move |res| {
            let string = format!("Transport to &main-color=#FF3333;{}&main-color=#89D6FF; active.\u{0}", ELEVATORS[layout[i] as usize].name);
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
        property_data: structs::SclyProperty::Timer(structs::Timer {
            name: b"Cutscene fixup timer\0".as_cstr(),

            start_time: 0.001,
            max_random_add: 0f32,
            reset_to_zero: 0,
            start_immediately: 1,
            active: 1,
        }),
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

fn patch_frigate_teleporter<'r>(area: &mut mlvl_wrapper::MlvlArea, spawn_room: SpawnRoom)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let wt = scly.layers.iter_mut()
        .flat_map(|layer| layer.objects.iter_mut())
        .find(|obj| obj.property_data.is_world_transporter())
        .and_then(|obj| obj.property_data.as_world_transporter_mut())
        .unwrap();
    wt.mlvl = spawn_room.mlvl;
    wt.mrea = spawn_room.mrea;
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
        property_data: structs::SclyProperty::Relay(structs::Relay {
            name: b"Relay Show Progress1\0".as_cstr(),
            active: 1,
        }),
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


fn patch_flaahgra_after_wild(ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
    -> Result<(), String>
{
    // Create a new layer. We want it to be active until Flaahgra is killed. It will contain
    // objects that renable the Flaahgra fight after the artifact is collect.
    area.add_layer(b"Post Wild Flaahgra\0".as_cstr());
    let scly = area.mrea().scly_section_mut();
    let new_layer_id = scly.layers.len() - 1;

    let disable_post_wild_flaahgra_id = ps.fresh_instance_id_range.next().unwrap();
    scly.layers.as_mut_vec()[1].objects.as_mut_vec().push(structs::SclyObject {
        instance_id: disable_post_wild_flaahgra_id,
        connections: vec![].into(),
        property_data: structs::SclyProperty::SpecialFunction(
            structs::SpecialFunction {
                name: b"Disable Post Wild Flaahgra\0".as_cstr(),
                position: [0., 0., 0.].into(),
                rotation: [0., 0., 0.].into(),
                type_: 16,
                unknown0: b"\0".as_cstr(),
                unknown1: 0.,
                unknown2: 0.,
                unknown3: 0.,
                layer_change_room_id: 0xf262c1ea,
                layer_change_layer_id: new_layer_id as u32,
                item_id: 0,
                unknown4: 1,
                unknown5: 0.,
                unknown6: 0xFFFFFFFF,
                unknown7: 0xFFFFFFFF,
                unknown8: 0xFFFFFFFF,
            }
        ),
    });
    let flaahgra_dead_relay = scly.layers.as_mut_vec()[1].objects.iter_mut()
        .find(|obj| obj.instance_id == 0x42500D4)
        .unwrap();
    flaahgra_dead_relay.connections.as_mut_vec().push(structs::Connection {
        state: structs::ConnectionState::ZERO,
        message: structs::ConnectionMsg::DECREMENT,
        target_object_id: disable_post_wild_flaahgra_id,
    });

    const LAYER_IDS: &[(u32, bool)] = &[(1, true), (2, false), (3, false)];
    let inst_ids: Vec<_> = (&mut ps.fresh_instance_id_range).take(LAYER_IDS.len()).collect();
    let new_layer = scly.layers.as_mut_vec().last_mut().unwrap();
    new_layer.objects.as_mut_vec().extend(LAYER_IDS.iter().zip(inst_ids.iter())
        .map(|((layer_id, _), inst_id)| structs::SclyObject {
            instance_id: *inst_id,
            connections: vec![].into(),
            property_data: structs::SclyProperty::SpecialFunction(
                structs::SpecialFunction {
                    name: b"Re-enable Post Wild Flaahgra\0".as_cstr(),
                    position: [0., 0., 0.].into(),
                    rotation: [0., 0., 0.].into(),
                    type_: 16,
                    unknown0: b"\0".as_cstr(),
                    unknown1: 0.,
                    unknown2: 0.,
                    unknown3: 0.,
                    layer_change_room_id: 0xf262c1ea,
                    layer_change_layer_id: *layer_id,
                    item_id: 0,
                    unknown4: 1,
                    unknown5: 0.,
                    unknown6: 0xFFFFFFFF,
                    unknown7: 0xFFFFFFFF,
                    unknown8: 0xFFFFFFFF,
                }
            ),
        }));

    let artifact_of_wild = scly.layers.as_mut_vec()[6].objects.iter_mut()
        .find(|obj| obj.instance_id == 0x18252f7d)
        .unwrap();
    artifact_of_wild.connections.as_mut_vec().extend(LAYER_IDS.iter().zip(inst_ids.iter())
        .map(|((_, enable), id)| structs::Connection {
            state: structs::ConnectionState::ARRIVED,
            message: if *enable { structs::ConnectionMsg::INCREMENT }
                           else { structs::ConnectionMsg::DECREMENT },
            target_object_id: *id,
        }));

    // Insert an intermediary relay between the trigger for the vines appearing and the vines
    // actually appearing so we can disable it after the artifact has been collected.
    let prevent_vines_relay_id = ps.fresh_instance_id_range.next().unwrap();
    scly.layers.as_mut_vec()[1].objects.as_mut_vec().push(structs::SclyObject {
        instance_id: prevent_vines_relay_id,
        property_data: structs::SclyProperty::Relay(
            structs::Relay {
                name: b"\0".as_cstr(),
                active: 1,
            }
        ),
        connections: vec![
            structs::Connection {
                state: structs::ConnectionState::ZERO,
                message: structs::ConnectionMsg::ACTIVATE,
                target_object_id: 0x2528f8,// Memory Relay - [IN] Turn On Vines
            }
        ].into()
    });
    let cinematic_trigger = scly.layers.as_mut_vec()[1].objects.iter_mut()
        .find(|obj| obj.instance_id == 0x4250064)
        .unwrap();
    cinematic_trigger.connections.as_mut_vec()
        .retain(|conn| conn.target_object_id != 0x2528f8);


    // Remove the object that deactivates the artifact layer
    scly.layers.as_mut_vec()[6].objects.as_mut_vec()
        .retain(|obj| obj.instance_id != 0x18253011);

    // After the artifact is collected a memory relay is activated. Use it to deactivate the relay
    // that enables the vines over the door.
    let artifact_collected_memory_relay = scly.layers.as_mut_vec()[6].objects.iter_mut()
        .find(|obj| obj.instance_id == 0x18253094)
        .unwrap();
    artifact_collected_memory_relay.connections.as_mut_vec().push(structs::Connection {
        state: structs::ConnectionState::ACTIVE,
        message: structs::ConnectionMsg::DEACTIVATE,
        target_object_id: prevent_vines_relay_id
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

fn make_elite_research_fight_prereq_patches(patcher: &mut PrimePatcher)
{
    patcher.add_scly_patch(b"metroid5.pak", 0x8A97BB54, |_ps, area| {
        let flags = &mut area.layer_flags.flags;
        *flags |= 1 << 1; // Turn on "3rd pass elite bustout"
        *flags &= !(1 << 5); // Turn off the "dummy elite"
        Ok(())
    });

    patcher.add_scly_patch(b"metroid5.pak", 0xFEA372E2, |_ps, area| {
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
        property_data: structs::SclyProperty:: SpecialFunction(structs::SpecialFunction {
                name: b"SpecialFunction - Remove Research Lab Aether wall\0".as_cstr(),
                position: [0., 0., 0.].into(),
                rotation: [0., 0., 0.].into(),
                type_: 16,
                unknown0: b"\0".as_cstr(),
                unknown1: 0.0,
                unknown2: 0.0,
                unknown3: 0.0,
                layer_change_room_id: 0x354889CE,
                layer_change_layer_id: 3,
                item_id: 0,
                unknown4: 1,
                unknown5: 0.0,
                unknown6: 0xFFFFFFFF,
                unknown7: 0xFFFFFFFF,
                unknown8: 0xFFFFFFFF
            }
        ),
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

fn patch_main_ventilation_shaft_section_b_door<'r>(
    ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea
)
    -> Result<(), String>
{
    let scly = area.mrea().scly_section_mut();
    let layer = &mut scly.layers.as_mut_vec()[0];

    layer.objects.as_mut_vec().push(structs::SclyObject {
        instance_id: ps.fresh_instance_id_range.next().unwrap(),
        property_data: structs::SclyProperty::Trigger(structs::Trigger {
                name: b"Trigger_DoorOpen-component\0".as_cstr(),
                position: [31.232622, 442.69165, -64.20529].into(),
                scale: [6.0, 17.0, 6.0].into(),
                damage_info: structs::structs::DamageInfo {
                    weapon_type: 0,
                    damage: 0.0,
                    radius: 0.0,
                    knockback_power: 0.0
                },
                unknown0: [0.0, 0.0, 0.0].into(),
                unknown1: 1,
                active: 1,
                unknown2: 0,
                unknown3: 0
            }),
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

fn patch_ore_processing_door_lock_0_02<'r>(_ps: &mut PatcherState, area: &mut mlvl_wrapper::MlvlArea)
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

fn patch_credits(res: &mut structs::Resource, pickup_layout: &[PickupType])
    -> Result<(), String>
{
    use std::fmt::Write;
    const PICKUPS_TO_PRINT: &[PickupType] = &[
        PickupType::ThermalVisor,
        PickupType::XRayVisor,
        PickupType::VariaSuit,
        PickupType::GravitySuit,
        PickupType::BoostBall,
        PickupType::SpiderBall,
        PickupType::ChargeBeam,
        PickupType::SpaceJumpBoots,
        PickupType::GrappleBeam,
        PickupType::SuperMissile,
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
        let room_name = pickup_meta::PICKUP_LOCATIONS.iter()
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


fn patch_starting_pickups(
    area: &mut mlvl_wrapper::MlvlArea,
    mut starting_items: u64,
    debug_print: bool,
) -> Result<(), String>
{

    let scly = area.mrea().scly_section_mut();
    let mut first = debug_print;
    macro_rules! print_maybe {
        ($first:ident, $($tts:tt)*) => {
            if $first {
                println!($($tts)*);
            }

        };
    }
    for layer in scly.layers.iter_mut() {
        for obj in layer.objects.iter_mut() {
            let spawn_point = if let Some(spawn_point) = obj.property_data.as_spawn_point_mut() {
                spawn_point
            } else {
                continue;
            };

            let mut fetch_bits = move |bits: u8| {
                let ret = starting_items & ((1 << bits) - 1);
                starting_items >>= bits;
                ret as u32
            };

            print_maybe!(first, "Starting pickups set:");

            spawn_point.scan_visor = 1;

            spawn_point.missiles = fetch_bits(8);
            print_maybe!(first, "    missiles: {}", spawn_point.missiles);

            spawn_point.energy_tanks = fetch_bits(4);
            print_maybe!(first, "    energy_tanks: {}", spawn_point.energy_tanks);

            spawn_point.power_bombs = fetch_bits(3);
            print_maybe!(first, "    power_bombs: {}", spawn_point.power_bombs);

            spawn_point.wave = fetch_bits(1);
            print_maybe!(first, "    wave: {}", spawn_point.wave);

            spawn_point.ice = fetch_bits(1);
            print_maybe!(first, "    ice: {}", spawn_point.ice);

            spawn_point.plasma = fetch_bits(1);
            print_maybe!(first, "    plasma: {}", spawn_point.plasma);

            spawn_point.charge = fetch_bits(1);
            print_maybe!(first, "    charge: {}", spawn_point.plasma);

            spawn_point.morph_ball = fetch_bits(1);
            print_maybe!(first, "    morph_ball: {}", spawn_point.morph_ball);

            spawn_point.bombs = fetch_bits(1);
            print_maybe!(first, "    bombs: {}", spawn_point.bombs);

            spawn_point.spider_ball = fetch_bits(1);
            print_maybe!(first, "    spider_ball: {}", spawn_point.spider_ball);

            spawn_point.boost_ball = fetch_bits(1);
            print_maybe!(first, "    boost_ball: {}", spawn_point.boost_ball);

            spawn_point.varia_suit = fetch_bits(1);
            print_maybe!(first, "    varia_suit: {}", spawn_point.varia_suit);

            spawn_point.gravity_suit = fetch_bits(1);
            print_maybe!(first, "    gravity_suit: {}", spawn_point.gravity_suit);

            spawn_point.phazon_suit = fetch_bits(1);
            print_maybe!(first, "    phazon_suit: {}", spawn_point.phazon_suit);

            spawn_point.thermal_visor = fetch_bits(1);
            print_maybe!(first, "    thermal_visor: {}", spawn_point.thermal_visor);

            spawn_point.xray= fetch_bits(1);
            print_maybe!(first, "    xray: {}", spawn_point.xray);

            spawn_point.space_jump = fetch_bits(1);
            print_maybe!(first, "    space_jump: {}", spawn_point.space_jump);

            spawn_point.grapple = fetch_bits(1);
            print_maybe!(first, "    grapple: {}", spawn_point.grapple);

            spawn_point.super_missile = fetch_bits(1);
            print_maybe!(first, "    super_missile: {}", spawn_point.super_missile);

            spawn_point.wavebuster = fetch_bits(1);
            print_maybe!(first, "    wavebuster: {}", spawn_point.super_missile);

            first = false;
        }
    }
    Ok(())
}

fn u32_to_be(x: u32) -> [u8; 4]
{
    let mut bytes = [0u8; 4];
    x.write_to(&mut io::Cursor::new(&mut bytes as &mut [u8])).unwrap();
    bytes
}

fn patch_dol<'r>(
    file: &mut structs::FstEntryFile,
    spawn_room: SpawnRoom,
    version: Version,
    patch_heat_damage: bool,
    patch_suit_damage: bool,
) -> Result<(), String>
{
    struct Addrs
    {
        load_mlvl_upper: u32,
        load_mlvl_lower: u32,
        load_mrea_idx: u32,
        disable_hints: u32,
        heat_damage_check: u32,
        suit_damage_reduction_calc: u32,
        save_filename_a: u32,
        save_filename_b: u32,
        cinematic_skip: u32,
        unlockables_default_ctor: u32,
        unlockables_read_ctor: u32,
    }

    let addrs = match version {
        Version::V0_00 => Addrs {
                load_mlvl_upper: 0x80022fbc,
                load_mlvl_lower: 0x80022fc8,
                heat_damage_check: 0x8014f784,
                load_mrea_idx: 0x801d5080,
                disable_hints: 0x8020f26c,
                suit_damage_reduction_calc: 0x80049efc,
                save_filename_a: 0x803d47cc,
                save_filename_b: 0x803d47db,
                cinematic_skip: 0x80151868,
                unlockables_read_ctor: 0x801d5c70,
                unlockables_default_ctor: 0x801d609c,
            },
        Version::V0_01 => return Err("Unreachable?".to_owned()),
        Version::V0_02 => Addrs {
                load_mlvl_upper: 0x800232a8,
                load_mlvl_lower: 0x800232b4,
                heat_damage_check: 0x8014ff68,
                load_mrea_idx: 0x801d58d0,
                disable_hints: 0x8020fae4,
                suit_damage_reduction_calc: 0x8004a1e8,
                save_filename_a: 0x803d588c,
                save_filename_b: 0x803d589b,
                cinematic_skip: 0x8015204c,
                unlockables_read_ctor: 0x801d64c0,
                unlockables_default_ctor: 0x801d68ec,
            },
    };


    let mrea_idx = spawn_room.mrea_idx;

    let mut mlvl_bytes = u32_to_be(spawn_room.mlvl);
    // PPC addi encoding shenanigans
    if mlvl_bytes[2] & 0x80 == 0x80 {
        mlvl_bytes[1] += 1;
    }

    let staggered_suit_damage_patch = ppcasm!(addrs.suit_damage_reduction_calc, {
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
            b       { addrs.suit_damage_reduction_calc as i32 + 0x9c };
        data:
            .float 0.0;
            .float 0.1;
            .float 0.2;
            .float 0.5;
    });

    let cinematic_skip_patch = ppcasm!(addrs.cinematic_skip, {
            li      r3, 0x1;
            blr;
    });
    let unlockables_default_ctor_patch = ppcasm!(addrs.unlockables_default_ctor, {
            li      r6, 100;
            stw     r6, 0xcc(r3);
            lis     r6, 0xF7FF;
            stw     r6, 0xd0(r3);
    });
    let unlockables_read_ctor_patch = ppcasm!(addrs.unlockables_read_ctor, {
            li      r6, 100;
            stw     r6, 0xcc(r28);
            lis     r6, 0xF7FF;
            stw     r6, 0xd0(r28);
            mr      r3, r29;
            li      r4, 2;
    });


    let heat_damage_patch = ppcasm!(addrs.heat_damage_check, {
            lwz     r4, 0xdc(r4);
            nop;
            subf    r0, r6, r5;
            cntlzw  r0, r0;
            nop;
    });


    let reader = match *file {
        structs::FstEntryFile::Unknown(ref reader) => reader.clone(),
        _ => panic!(),
    };

    let mut dol_patcher = DolPatcher::new(reader);
    dol_patcher
        .patch(addrs.load_mlvl_upper + 2, Cow::Owned(vec![mlvl_bytes[0], mlvl_bytes[1]]))?
        .patch(addrs.load_mlvl_lower + 2, Cow::Owned(vec![mlvl_bytes[2], mlvl_bytes[3]]))?
        .patch(addrs.load_mrea_idx + 3, Cow::Owned(vec![mrea_idx as u8]))?
        .patch(addrs.disable_hints + 1, Cow::Borrowed(&[0xC0u8] as &[u8]))?
        .patch(addrs.save_filename_a, b"randomprime A\0"[..].into())?
        .patch(addrs.save_filename_b, b"randomprime B\0"[..].into())?
        .ppcasm_patch(&cinematic_skip_patch)?
        .ppcasm_patch(&unlockables_default_ctor_patch)?
        .ppcasm_patch(&unlockables_read_ctor_patch)?;

    if patch_heat_damage {
        dol_patcher.ppcasm_patch(&heat_damage_patch)?;
    }

    if patch_suit_damage {
        dol_patcher.ppcasm_patch(&staggered_suit_damage_patch)?;
    }

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
    let res = pickup_meta::build_resource(
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

    write_encoded_str("game_name", &config.bnr_game_name, &mut bnr.game_name)?;
    write_encoded_str("developer", &config.bnr_developer, &mut bnr.developer)?;
    write_encoded_str("game_name_full", &config.bnr_game_name_full, &mut bnr.game_name_full)?;
    write_encoded_str("developer_full", &config.bnr_developer_full, &mut bnr.developer_full)?;
    write_encoded_str("description", &config.bnr_description, &mut bnr.description)?;

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

pub struct ParsedConfig
{
    pub input_iso: memmap::Mmap,
    pub output_iso: File,
    pub layout_string: String,

    pub pickup_layout: Vec<u8>,
    pub elevator_layout: Vec<u8>,
    pub seed: [u32; 16],

    pub iso_format: IsoFormat,
    pub skip_frigate: bool,
    pub skip_hudmenus: bool,
    pub keep_fmvs: bool,
    pub obfuscate_items: bool,
    pub nonvaria_heat_damage: bool,
    pub staggered_suit_damage: bool,
    pub quiet: bool,

    pub artifact_hint_behavior: ArtifactHintBehavior,

    pub flaahgra_music_files: Option<[nod_wrapper::FileWrapper; 2]>,

    pub starting_items: Option<u64>,
    pub comment: String,

    pub bnr_game_name: Option<String>,
    pub bnr_developer: Option<String>,

    pub bnr_game_name_full: Option<String>,
    pub bnr_developer_full: Option<String>,
    pub bnr_description: Option<String>,
}


#[derive(PartialEq, Copy, Clone)]
enum Version
{
    V0_00,
    V0_01,
    V0_02,
}

pub fn patch_iso<T>(config: ParsedConfig, mut pn: T) -> Result<(), String>
    where T: structs::ProgressNotifier
{
    let mut ct = Vec::new();
    writeln!(ct, "Created by randomprime version {}", env!("CARGO_PKG_VERSION")).unwrap();
    writeln!(ct).unwrap();
    writeln!(ct, "Options used:").unwrap();
    writeln!(ct, "configuration string: {}", config.layout_string).unwrap();
    writeln!(ct, "skip frigate: {}", config.skip_frigate).unwrap();
    writeln!(ct, "keep fmvs: {}", config.keep_fmvs).unwrap();
    writeln!(ct, "nonmodal hudmemos: {}", config.skip_hudmenus).unwrap();
    writeln!(ct, "obfuscated items: {}", config.obfuscate_items).unwrap();
    writeln!(ct, "{}", config.comment).unwrap();

    let mut reader = Reader::new(unsafe { config.input_iso.as_slice() });

    let mut gc_disc: structs::GcDisc = reader.read(());

    if &gc_disc.header.game_identifier() != b"GM8E01" {
        Err("The input ISO doesn't appear to be NTSC-US Metroid Prime.".to_string())?
    }
    if gc_disc.find_file("randomprime.txt").is_some() {
        Err(concat!("The input ISO has already been randomized once before. ",
                    "You must start from an unmodified ISO every time."
        ))?
    }
    let version = match (gc_disc.header.disc_id, gc_disc.header.version) {
        (0, 0) => Version::V0_00,
        (0, 1) => Version::V0_01,
        (0, 2) => Version::V0_02,
        (a, b) => Err(format!("Unknown game version {}-{}", a, b))?
    };
    if config.skip_frigate && version == Version::V0_01 {
        Err(concat!("The frigate level skip is not currently supported for the ",
                    "0-01 version of Metroid Prime").to_string())?;
    }


    build_and_run_patches(&mut gc_disc, &config, version)?;

    gc_disc.file_system_table.add_file(
        b"randomprime.txt\0".as_cstr(),
        structs::FstEntryFile::Unknown(Reader::new(&ct)),
    );

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

fn build_and_run_patches(gc_disc: &mut structs::GcDisc, config: &ParsedConfig, version: Version) -> Result<(), String>
{
    let pickup_layout: Vec<_> = config.pickup_layout.iter()
        .map(|i| PickupType::from_idx(*i as usize).unwrap())
        .collect();
    let pickup_layout = &pickup_layout[..];

    let mut rng = ChaChaRng::from_seed(&config.seed);
    let artifact_totem_strings = build_artifact_temple_totem_scan_strings(pickup_layout, &mut rng);

    let mut pickup_resources = collect_pickup_resources(gc_disc);
    if config.skip_hudmenus {
        add_skip_hudmemos_strgs(&mut pickup_resources);
    }

    // XXX These values need to outlife the patcher
    let select_game_fmv_suffix = rng.choose(&["A", "B", "C"]).unwrap();
    let n = format!("02_start_fileselect_{}.thp", select_game_fmv_suffix);
    let start_file_select_fmv = gc_disc.find_file(&n).unwrap().file.clone();
    let n = format!("04_fileselect_playgame_{}.thp", select_game_fmv_suffix);
    let file_select_play_game_fmv = gc_disc.find_file(&n).unwrap().file.clone();

    let pickup_resources = &pickup_resources;
    let mut patcher = PrimePatcher::new();
    patcher.add_file_patch(b"opening.bnr", |file| patch_bnr(file, config));
    if !config.keep_fmvs {
        // Replace the attract mode FMVs with empty files to reduce the amount of data we need to
        // copy and to make compressed ISOs smaller.
        const FMV_NAMES: &[&[u8]] = &[
            b"attract0.thp",
            b"attract1.thp",
            b"attract2.thp",
            b"attract3.thp",
            b"attract4.thp",
            b"attract5.thp",
            b"attract6.thp",
            b"attract7.thp",
            b"attract8.thp",
            b"attract9.thp",
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
            b"rui_flaaghraR.dsp",
            b"rui_flaaghraL.dsp",
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
        b"02_start_fileselect_A.thp",
        b"02_start_fileselect_B.thp",
        b"02_start_fileselect_C.thp",
        b"04_fileselect_playgame_A.thp",
        b"04_fileselect_playgame_B.thp",
        b"04_fileselect_playgame_C.thp",
    ];
    for fmv_name in SELECT_GAMES_FMVS {
        let fmv_ref = if fmv_name[1] == b'2' {
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
    for (name, rooms) in pickup_meta::PICKUP_LOCATIONS.iter() {
        for room_info in rooms.iter() {
             patcher.add_scly_patch(name.as_bytes(), room_info.room_id, move |_, area| {
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
                patcher.add_scly_patch(
                    name.as_bytes(),
                    room_info.room_id,
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

    let spawn_room = SpawnRoom::from_room_idx(config.elevator_layout[20] as usize);
    if config.skip_frigate {
        patcher.add_file_patch(
            b"default.dol",
            move |file| patch_dol(
                file,
                spawn_room,
                version,
                config.nonvaria_heat_damage,
                config.staggered_suit_damage,
            )
        );
        patcher.add_file_patch(b"Metroid1.pak", empty_frigate_pak);
    } else {
        patcher.add_file_patch(
            b"default.dol",
            |file| patch_dol(
                file,
                SpawnRoom::frigate_spawn_room(),
                version,
                config.nonvaria_heat_damage,
                config.staggered_suit_damage,
            )
        );
        patcher.add_scly_patch(
            b"Metroid1.pak",
            0xD1241219,
            move |_ps, area| patch_frigate_teleporter(area, spawn_room)
        );
    }

    let (starting_items, print_sis) = if let Some(starting_items) = config.starting_items {
        (starting_items, true)
    } else {
        (0, false)
    };
    patcher.add_scly_patch(
        spawn_room.pak_name.as_bytes(),
        spawn_room.mrea,
        move |_ps, area| patch_starting_pickups(area, starting_items, print_sis)
    );

    // TODO: It might be nice for this list to be generated by resource_tracing, but
    //       the sorting is probably non-trivial.
    const ARTIFACT_TOTEM_SCAN_STRGS: &[u32] = &[
        0x61729798,// Lifegiver
        0xAA2E443D,// Wild
        0x8E9C7387,// World
        0x16B057E3,// Sun
        0xB72B7485,// Elder
        0x45C0A022,// Spirit
        0xFAE3D58E,// Truth
        0x2CBA3693,// Chozo
        0xE7E6E536,// Warrior
        0xC354D28C,// Newborn
        0xDDEC8446,// Nature
        0x7C77A720,// Strength
    ];
    for (file_id, strg_text) in ARTIFACT_TOTEM_SCAN_STRGS.iter().zip(artifact_totem_strings.iter()) {
        patcher.add_resource_patch(
            b"Metroid4.pak",
            b"STRG".into(),
            *file_id,
            move |res| patch_artifact_totem_scan_strg(res, &strg_text),
        );
    }

    patcher.add_resource_patch(
        b"NoARAM.pak",
        b"STRG".into(),
        0x324D0835,
        |res| patch_credits(res, &pickup_layout)
    );

    patcher.add_resource_patch(b"Metroid4.pak", b"SAVW".into(), asset_ids::PHAZON_MINES_SAVW,
                               patch_mines_savw_for_phazon_suit_scan);
    patcher.add_scly_patch(
        b"Metroid4.pak",
        asset_ids::ARTIFACT_TEMPLE_MREA,
        |ps, area| fix_artifact_of_truth_requirements(ps, area, &pickup_layout)
    );
    patcher.add_scly_patch(
        b"Metroid4.pak",
        asset_ids::ARTIFACT_TEMPLE_MREA,
        |ps, area| patch_artifact_hint_availability(ps, area, config.artifact_hint_behavior)
    );

    patcher.add_resource_patch(b"NoARAM.pak", b"TXTR".into(), 0x4CAD3BCC, patch_save_banner_txtr);

    make_elevators_patch(&mut patcher, &config.elevator_layout);

    make_elite_research_fight_prereq_patches(&mut patcher);

    patcher.add_scly_patch(b"Metroid2.pak", 0x9A0A03EB, patch_flaahgra_after_wild);
    patcher.add_scly_patch(b"Metroid4.pak", 0xBDB1FCAC, patch_temple_security_station_cutscene_trigger);
    patcher.add_scly_patch(b"Metroid4.pak", 0xAFD4E038, patch_main_ventilation_shaft_section_b_door);
    patcher.add_scly_patch(b"Metroid3.pak", 0x43E4CC25, patch_research_lab_hydra_barrier);
    patcher.add_scly_patch(b"Metroid3.pak", 0xA49B2544, patch_research_lab_aether_exploding_wall);
    patcher.add_scly_patch(b"Metroid3.pak", 0x3FB4A34E, patch_observatory_2nd_pass_solvablility);
    patcher.add_scly_patch(b"metroid5.pak", 0x956F1552, patch_mines_security_station_soft_lock);


    if version == Version::V0_02 {
        patcher.add_scly_patch(b"metroid5.pak", 0x643d038f, patch_ore_processing_door_lock_0_02);
        patcher.add_scly_patch(b"Metroid6.pak", 0xc0498676, patch_geothermal_core_door_lock_0_02);
    }

    if config.elevator_layout[20] != 20 {
        // If we have a non-default start point, patch the landing site to avoid
        // weirdness with cutscene triggers and the ship spawning.
        patcher.add_scly_patch(b"Metroid4.pak", 0xB2701146, patch_landing_site_cutscene_triggers);
    }
    patcher.run(gc_disc)?;
    Ok(())
}

