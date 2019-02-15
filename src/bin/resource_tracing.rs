//! This program traces the dependencies of each pickup in a Metroid Prime ISO.
//! The location of the ISO should be provided as a command line argument.

pub use randomprime::*;
use randomprime::pickup_meta::ScriptObjectLocation;

use reader_writer::{FourCC, Reader, Writable};
use structs::{Ancs, Cmdl, Evnt, Pickup, Scan, Resource};

use std::{
    mem,
    env::args,
    fs::File,
    borrow::Cow,
    collections::{HashMap, HashSet},
    ffi::CStr,
    str as stdstr,
};

// Duplicated from pickup_meta. This version needs owned-lists instead of borrowed.
#[derive(Clone, Debug)]
pub struct PickupLocation
{
    location: ScriptObjectLocation,
    hudmemo: ScriptObjectLocation,
    attainment_audio: ScriptObjectLocation,
    post_pickup_relay_connections: Vec<structs::Connection>,
}

struct ResourceDb<'a>
{
    map: HashMap<ResourceKey, ResourceDbRecord<'a>>,
}

struct ResourceDbRecord<'a>
{
    data: ResourceData<'a>,
    deps: Option<HashSet<ResourceKey>>,
}

impl<'a> ResourceDb<'a>
{
    fn new() -> ResourceDb<'a>
    {
        ResourceDb {
            map: HashMap::new(),
        }
    }

    fn add_resource(&mut self, res: Resource<'a>)
    {
        let key = ResourceKey::new(res.file_id, res.fourcc());
        self.map.entry(key).or_insert_with(move || {
            ResourceDbRecord {
                data: ResourceData::new(&res),
                deps: None,
            }
        });
    }

    fn get_dependencies(&mut self, pickup: &Pickup) -> HashSet<ResourceKey>
    {
        let base_resources = [
            (ResourceKey::new(pickup.cmdl, b"CMDL".into()), None),
            (ResourceKey::new(pickup.ancs.file_id, b"ANCS".into()), Some(pickup.ancs.node_index)),
            (ResourceKey::new(pickup.actor_params.scan_params.scan, b"SCAN".into()), None),
            (ResourceKey::new(pickup.actor_params.xray_cmdl, b"CMDL".into()), None),
            (ResourceKey::new(pickup.actor_params.xray_cskr, b"CSKR".into()), None),
            (ResourceKey::new(pickup.part, b"PART".into()), None),
        ];
        let mut result = HashSet::new();
        for r in base_resources.iter() {
            self.extend_set_with_deps(&mut result, r.0, r.1);
        };
        result
    }

    // The output has been tailored to match the observed behavior of Claris's
    // randomizer.
    // A few sections of code are commented out, indicating what appear to me to
    // be dependencies, but don't seem to match Claris's dependency lists.
    fn get_resource_deps(&mut self, key: ResourceKey, ancs_node: Option<u32>) -> HashSet<ResourceKey>
    {
        let mut deps = HashSet::with_capacity(0);

        let data = {
            let ref record = self.map[&key];
            if let Some(ref deps) = record.deps {
                return deps.clone();
            };
            record.data.clone()
        };
        {
            // To avoid line-wrapping, create a "specialized" version of the method.
            let mut extend_deps = |id, b: &[u8; 4]| {
                self.extend_set_with_deps(&mut deps, ResourceKey::new(id, b.into()), None);
            };

            if key.fourcc == b"SCAN".into() {
                let scan: Scan = data.data.clone().read(());
                extend_deps(scan.frme, b"FRME");
                extend_deps(scan.strg, b"STRG");
            } else if key.fourcc == b"EVNT".into() {
                let evnt: Evnt = data.data.clone().read(());
                for effect in evnt.effect_events.iter() {
                    extend_deps(effect.effect_file_id, effect.effect_type.as_bytes());
                }
            } else if key.fourcc == b"PART".into() {
                let buf = data.decompress();
                let buf: &[u8] = &buf;
                // We're cheating here. We're going to find the sub-string ICTSCNST
                // and then using the next word as the id of a PART.
                for i in 0..(buf.len() - 8) {
                    if &buf[i..(i + 8)] == b"ICTSCNST" {
                        let id : u32 = Reader::new(&buf[(i + 8)..(i+12)]).read(());
                        if id != 0 {
                            extend_deps(id, b"PART");
                        }
                        // TODO: IITS and IDTS too?
                    } else if &buf[i..(i + 4)] == b"TEXR" {
                        if &buf[(i + 4)..(i + 8)] == b"ATEX" {
                            let id : u32 = Reader::new(&buf[(i + 12)..(i + 16)]).read(());
                            if id != 0 {
                                extend_deps(id, b"TXTR");
                            }
                        }
                    } else if &buf[i..(i + 4)] == b"KSSM" && &buf[(i + 4)..(i + 8)] != b"NONE" {

                        let kssm : structs::Kssm = Reader::new(&buf[(i + 8)..]).read(());
                        for list in kssm.lists.iter() {
                            for item in list.items.iter() {
                                extend_deps(item.part, b"PART");
                            }
                        }
                    }
                }
            } else if key.fourcc == b"CMDL".into() {
                let buf = data.decompress();
                let cmdl: Cmdl = Reader::new(&buf).read(());
                for material in cmdl.material_sets.iter() {
                    for id in material.texture_ids.iter() {
                        extend_deps(id, b"TXTR");
                    }
                }
            } else if key.fourcc == b"ANCS".into() {
                let buf = data.decompress();
                let ancs: Ancs = Reader::new(&buf).read(());
                if let Some(ancs_node) = ancs_node {
                    let char_info = ancs.char_set.char_info.iter().nth(ancs_node as usize).unwrap();
                    extend_deps(char_info.cmdl, b"CMDL");
                    extend_deps(char_info.cskr, b"CSKR");
                    extend_deps(char_info.cinf, b"CINF");
                    // char_info.effects.map(|effects| for effect in effects.iter() {
                    //     for comp in effect.components.iter() {
                    //         extend_deps(ResourceKey::new(comp.file_id, comp.type_));
                    //     }
                    // });
                    // char_info.overlay_cmdl.map(|cmdl| extend_deps(cmdl, b"CMDL"));
                    // char_info.overlay_cskr.map(|cmdl| extend_deps(cmdl, b"CSKR"));
                    for part in char_info.particles.part_assets.iter() {
                        extend_deps(part, b"PART");
                    }
                };
                ancs.anim_set.animation_resources.map(|i| for anim_resource in i.iter() {
                    extend_deps(anim_resource.anim, b"ANIM");
                    extend_deps(anim_resource.evnt, b"EVNT");
                });
            }
        }

        // We can't safely cache the result if we are using a specific ANCS node.
        // XXX This would be fine if the data structure implementing the cache was
        //     reworked.
        if ancs_node.is_none() {
            self.map.get_mut(&key).unwrap().deps = Some(deps.clone());
        }
        deps
    }

    fn extend_set_with_deps(&mut self, set: &mut HashSet<ResourceKey>, key: ResourceKey,
                                       ancs_node: Option<u32>)
    {
        if key.file_id != u32::max_value() {
            set.insert(key);
            set.extend(self.get_resource_deps(key, ancs_node));
        };
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct ResourceKey
{
    file_id: u32,
    fourcc: FourCC
}

impl ResourceKey
{
    fn new(file_id: u32, fourcc: FourCC) -> ResourceKey
    {
        ResourceKey {
            file_id: file_id,
            fourcc: fourcc,
        }
    }
}


pub struct PickupType
{
    pub name: &'static str,
    pub first_loc: usize,
}
// A map from pickup type -> pickup name and location
// Note, the ordering matters here
const PICKUP_TYPES: &'static [PickupType] = &[
    PickupType { name: "Missile", first_loc: 1 },
    PickupType { name: "Energy Tank", first_loc: 9, },

    PickupType { name: "Thermal Visor", first_loc: 50, },
    PickupType { name: "X-Ray Visor", first_loc: 71, },

    PickupType { name: "Varia Suit", first_loc: 20, },
    PickupType { name: "Gravity Suit", first_loc: 54, },
    PickupType { name: "Phazon Suit", first_loc: 83, },

    PickupType { name: "Morph Ball", first_loc: 5, },
    PickupType { name: "Boost Ball", first_loc: 43, },
    PickupType { name: "Spider Ball", first_loc: 44, },

    PickupType { name: "Morph Ball Bomb", first_loc: 28, },
    PickupType { name: "Power Bomb Expansion", first_loc: 12, },
    PickupType { name: "Power Bomb", first_loc: 85, },

    PickupType { name: "Charge Beam", first_loc: 23, },
    PickupType { name: "Space Jump Boots", first_loc: 59, },
    PickupType { name: "Grapple Beam", first_loc: 75, },

    PickupType { name: "Super Missile", first_loc: 47, },
    PickupType { name: "Wavebuster", first_loc: 13, },
    PickupType { name: "Ice Spreader", first_loc: 96, },
    PickupType { name: "Flamethrower", first_loc: 76, },

    PickupType { name: "Wave Beam", first_loc: 41, },
    PickupType { name: "Ice Beam", first_loc: 34, },
    PickupType { name: "Plasma Beam", first_loc: 99, },

    PickupType { name: "Artifact of Lifegiver", first_loc: 14, },
    PickupType { name: "Artifact of Wild", first_loc: 21, },
    PickupType { name: "Artifact of World", first_loc: 33, },
    PickupType { name: "Artifact of Sun", first_loc: 37, },
    PickupType { name: "Artifact of Elder", first_loc: 49, },
    PickupType { name: "Artifact of Spirit", first_loc: 56, },
    PickupType { name: "Artifact of Truth", first_loc: 63, },
    PickupType { name: "Artifact of Chozo", first_loc: 72, },
    PickupType { name: "Artifact of Warrior", first_loc: 77, },
    PickupType { name: "Artifact of Newborn", first_loc: 89, },
    PickupType { name: "Artifact of Nature", first_loc: 91, },
    PickupType { name: "Artifact of Strength", first_loc: 95, },
];

static CUT_SCENE_PICKUPS: &'static [(u32, u32)] = &[
    (0x3C785450, 589860), // Morph Ball
    (0x0D72F1F7, 1377077), // Wavebuster
    (0x11BD63B7, 1769497), // Artifact of Lifegiver
    (0xC8309DF6, 2359772), // Missile Launcher
    (0x9A0A03EB, 2435310), // Varia Suit
    (0x9A0A03EB, 405090173), // Artifact of Wild
    (0x492CBF4A, 2687109), // Charge Beam
    (0x4148F7B0, 3155850), // Morph Ball Bomb
    (0xE1981EFC, 3735555), // Artifact of World
    (0xAFEFE677, 3997699), // Ice Beam

    // XXX Doesn't normally have a cutscene. Skip?
    (0x6655F51E, 524887), // Artifact of Sun

    (0x40C548E9, 917592), // Wave Beam
    (0xA20A7455, 1048801), // Boost Ball
    (0x70181194, 1573322), // Spider Ball
    (0x3FB4A34E, 1966838), // Super Missile

    // XXX Doesn't normally have a cutscene. Skip?
    (0xB3C33249, 2557135), // Artifact of Elder

    (0xA49B2544, 69730588), // Thermal Visor
    (0x49175472, 3473439), // Gravity Suit
    (0xF7C84340, 3539113), // Artifact of Spirit
    (0xC44E7A07, 262151), // Space Jump Boots
    (0x2398E906, 68157908), // Artifact of Truth
    (0x86EB2E02, 2752545), // X-Ray Visor

    // XXX Doesn't normally have a cutscene. Skip?
    (0x86EB2E02, 2753076), // Artifact of Chozo

    (0xE39C342B, 589827), // Grapple Beam
    (0x35C5D736, 786470), // Flamethrower !!!!

    // XXX Doesn't normally have a cutscene. Skip?
    (0x8A97BB54, 852800), // Artifact of Warrior

    // XXX Doesn't normally have a cutscene. Skip?
    (0xBBFA4AB3, 2556031), // Artifact of Newborn

    (0xA4719C6A, 272508), // Artifact of Nature

    // XXX Doesn't normally have a cutscene. Skip?
    (0x89A6CB8D, 720951), // Artifact of Strength

    (0x901040DF, 786472), // Ice Spreader
    (0x4CC18E5A, 1376287), // Plasma Beam
];


#[derive(Debug)]
struct PickupData
{
    name: &'static str,
    bytes: Vec<u8>,
    deps: HashSet<ResourceKey>,
    hudmemo_strg: u32,
    attainment_audio_file_name: Vec<u8>,
}

#[derive(Debug)]
struct RoomInfo
{
    room_id: u32,
    name: String,
    pickups: Vec<PickupLocation>,
    objects_to_remove: HashMap<u32, Vec<u32>>,
}


fn trace_pickup_deps(
    gc_disc: &mut structs::GcDisc,
    pak_name: &str,
    counter: &mut usize,
    pickup_table: &mut HashMap<usize, PickupData>,
    locations: &mut Vec<Vec<RoomInfo>>,
    cmdl_aabbs: &mut HashMap<u32, [f32; 6]>,
)
{
    let file_entry = gc_disc.find_file(pak_name).unwrap();
    let pak = match *file_entry.file().unwrap() {
        structs::FstEntryFile::Pak(ref pak) => pak.clone(),
        structs::FstEntryFile::Unknown(ref reader) => reader.clone().read(()),
        _ => panic!(),
    };

    let resources = &pak.resources;

    let mut res_db = ResourceDb::new();
    for res in resources.iter() {
        res_db.add_resource(res.into_owned());
    }

    let mrea_name_strg_map: HashMap<_, _> = resources.iter()
        .find(|res| res.fourcc() == b"MLVL".into())
        .unwrap()
        .kind.as_mlvl().unwrap()
        .areas.iter()
        .map(|area| (area.mrea, area.area_name_strg))
        .collect();


    locations.push(vec![]);
    let locations = locations.last_mut().unwrap();

    for res in resources.iter() {
        if res.fourcc() != b"MREA".into() {
            continue;
        };

        let mut res = res.into_owned();
        let mrea = res.kind.as_mrea_mut().unwrap();

        let scly = mrea.sections.iter()
            .nth(mrea.scly_section_idx as usize)
            .unwrap();
        let scly: structs::Scly = match *scly {
            structs::MreaSection::Unknown(ref reader) => reader.clone().read(()),
            structs::MreaSection::Scly(ref scly) => scly.clone(),
        };

        let mut pickups = vec![];
        let mut scly_db = HashMap::new();
        for (layer_num, scly_layer) in scly.layers.iter().enumerate() {
            for obj in scly_layer.objects.iter() {
                let obj = obj.into_owned();
                if let Some(pickup) = obj.property_data.as_pickup() {
                    // We're only interested in "real" pickups
                    if pickup.max_increase > 0 {
                        pickups.push((layer_num, obj.clone()));

                        if pickup.cmdl != u32::max_value() {
                            // Add an aabb entry for this pickup's cmdl
                            cmdl_aabbs.entry(pickup.cmdl).or_insert_with(|| {
                                let cmdl_key = ResourceKey::new(pickup.cmdl, b"CMDL".into());
                                // Cmdls are compressed
                                let res_data = res_db.map[&cmdl_key].data.decompress();
                                let cmdl: Cmdl = Reader::new(&res_data).read(());
                                let aabb = cmdl.maab;
                                // Convert from GenericArray to [f32; 6]
                                [aabb[0], aabb[1], aabb[2], aabb[3], aabb[4], aabb[5]]
                            });
                        }
                    }
                }
                assert!(scly_db.insert(obj.instance_id, (layer_num, obj)).is_none());
            }
        }

        for (layer_num, mut obj) in pickups {
            if *counter == 84 {
                // Skip the extra phazon suit-thing
                *counter += 1;
                continue
            }

            let pickup = obj.property_data.as_pickup_mut().unwrap();
            let mut deps = res_db.get_dependencies(&pickup);
            patch_dependencies(pickup.kind, &mut deps);

            const ATTAINMENT_AUDIO_FILES: &'static [&'static [u8]] = &[
                b"/audio/itm_x_short_02.dsp",
                b"audio/jin_artifact.dsp",
                b"audio/jin_itemattain.dsp",
            ];
            let attainment_audio = search_for_scly_object(&obj.connections, &scly_db,
                |obj| obj.property_data.as_streamed_audio()
                    .map(|sa| ATTAINMENT_AUDIO_FILES.contains(&sa.audio_file_name.to_bytes()))
                    .unwrap_or(false)
            );

            let attainment_audio_location;
            let attainment_audio_file_name;
            if let Some(attainment_audio) = attainment_audio {
                attainment_audio_location = ScriptObjectLocation {
                    layer: scly_db[&attainment_audio.instance_id].0 as u32,
                    instance_id: attainment_audio.instance_id,
                };
                let streamed_audio = attainment_audio.property_data.as_streamed_audio().unwrap();
                attainment_audio_file_name = streamed_audio.audio_file_name.to_bytes_with_nul()
                                                           .to_owned();
            } else {
                // The Phazon Suit is weird: the audio object isn't directly connected to the
                // Pickup. So, hardcode its location.
                assert!(*counter == 83);
                attainment_audio_location = ScriptObjectLocation {
                    layer: 1,
                    instance_id: 68813644,
                };
                attainment_audio_file_name = b"audio/jin_itemattain.dsp\0".to_vec();
            };

            let hudmemo_loc;
            let hudmemo_strg;
            let hudmemo = search_for_scly_object(&obj.connections, &scly_db,
                |obj| obj.property_data.as_hud_memo()
                    .map(|hm| hm.name.to_str().unwrap().contains("Pickup"))
                    .unwrap_or(false)
            );
            if let Some(hudmemo) = hudmemo {
                let strg = hudmemo.property_data.as_hud_memo().unwrap().strg;
                hudmemo_strg = strg;
                hudmemo_loc = ScriptObjectLocation {
                    layer: scly_db[&hudmemo.instance_id].0 as u32,
                    instance_id: hudmemo.instance_id,
                };
            } else {
                // Overrides for the Phazon Suit
                assert_eq!(pickup.kind, 23);
                hudmemo_strg = asset_ids::PHAZON_SUIT_ACQUIRED_HUDMEMO_STRG;
                pickup.actor_params.scan_params.scan = asset_ids::PHAZON_SUIT_SCAN;

                pickup.cmdl = asset_ids::PHAZON_SUIT_CMDL;
                pickup.ancs.file_id = asset_ids::PHAZON_SUIT_ANCS;

                hudmemo_loc = ScriptObjectLocation {
                    layer: scly_db[&68813640].0 as u32,
                    instance_id: 68813640,
                };
            }

            let mut removals = Vec::new();
            if pickup.kind >= 29 && pickup.kind <= 40 {
                // If this is an artifact...
                let layer_switch_function = search_for_scly_object(&obj.connections, &scly_db,
                        |obj| obj.property_data.as_special_function()
                            .map(|hm| hm.name.to_str().unwrap()
                                    == "SpecialFunction ScriptLayerController -- Stonehenge Totem")
                            .unwrap_or(false),
                    ).unwrap();
                removals.push(ScriptObjectLocation {
                    layer: scly_db[&layer_switch_function.instance_id].0 as u32,
                    instance_id: layer_switch_function.instance_id,
                });

                let pause_function = search_for_scly_object(&obj.connections, &scly_db,
                        |obj| obj.property_data.as_special_function()
                            .map(|hm| hm.name.to_str().unwrap()
                                    == "SpecialFunction - Enter Logbook Screen")
                            .unwrap_or(false),
                    ).unwrap();
                removals.push(ScriptObjectLocation {
                    layer: scly_db[&pause_function.instance_id].0 as u32,
                    instance_id: pause_function.instance_id,
                });
            }

            // Remove the PlayerHint objects that disable control when collecting an item.
            let player_hint = search_for_scly_object(&obj.connections, &scly_db,
                    |obj| obj.property_data.as_player_hint()
                        .map(|hm| hm.name.to_str().unwrap() == "Player Hint Disable Controls")
                        .unwrap_or(false),
                );
            if let Some(player_hint) = player_hint {
                removals.push(ScriptObjectLocation {
                    layer: scly_db[&player_hint.instance_id].0 as u32,
                    instance_id: player_hint.instance_id,
                });
            };

            // If this is a pickup with an associated cutscene, find the connections we want to
            // preserve and the objects we want to remove.
            let mut post_pickup_relay_connections = vec![];
            if CUT_SCENE_PICKUPS.contains(&(res.file_id, obj.instance_id)) {
                post_pickup_relay_connections = build_skip_cutscene_relay_connections(
                    pickup.kind, &obj.connections, &scly_db);
                removals.push(find_cutscene_trigger_relay(pickup.kind, &obj.connections, &scly_db))
            }

            if let Some(kind_id) = PICKUP_TYPES.iter().position(|pt| *counter == pt.first_loc) {
                let mut data = vec![];
                pickup.write(&mut data).unwrap();
                pickup_table.insert(kind_id, PickupData {
                    name: PICKUP_TYPES[kind_id].name,
                    bytes: data,
                    deps: deps,
                    hudmemo_strg: hudmemo_strg,
                    attainment_audio_file_name: attainment_audio_file_name,
                });
            }

            let location = PickupLocation {
                location: ScriptObjectLocation {
                    layer: layer_num as u32,
                    instance_id: obj.instance_id,
                },
                attainment_audio: attainment_audio_location,
                hudmemo: hudmemo_loc,
                post_pickup_relay_connections: post_pickup_relay_connections,
            };
            let fid = res.file_id; // Ugh, the borrow checker...
            if locations.last().map(|i| i.room_id == fid).unwrap_or(false) {
                locations.last_mut().unwrap().pickups.push(location);
            } else {
                let strg_id = mrea_name_strg_map[&res.file_id];
                let strg: structs::Strg = res_db.map[&ResourceKey::new(strg_id, b"STRG".into())]
                    .data.data.clone().read(());
                let name = strg
                    .string_tables.iter().next().unwrap()
                    .strings.iter().next().unwrap()
                    .into_owned().into_string();
                locations.push(RoomInfo {
                    room_id: res.file_id,
                    name: name,
                    pickups: vec![location],
                    objects_to_remove: HashMap::new(),
                });
            }
            let objects_to_remove = &mut locations.last_mut().unwrap().objects_to_remove;
            for r in removals {
                objects_to_remove.entry(r.layer).or_insert_with(Vec::new).push(r.instance_id);
            }

            *counter += 1;
        }
    }
}

fn search_for_scly_object<'a, F>(
    connections: &reader_writer::LazyArray<'a, structs::Connection>,
    scly_db: &HashMap<u32, (usize, structs::SclyObject<'a>)>,
    f: F
) -> Option<structs::SclyObject<'a>>
    where F: Fn(&structs::SclyObject<'a>) -> bool
{
    let mut stack = Vec::new();

    // Circular references are possible, so keep track of which ones we've seen
    // already.
    let mut seen = HashSet::new();

    for c in connections {
        stack.push(c.target_object_id);
        seen.insert(c.target_object_id);
    }

    while let Some(id) = stack.pop() {
        let obj = if let Some(&(_, ref obj)) = scly_db.get(&id) {
            obj
        } else {
            continue;
        };
        if f(&obj) {
            return Some(obj.clone());
        }
        for c in obj.connections.iter() {
            if !seen.contains(&c.target_object_id) {
                stack.push(c.target_object_id);
                seen.insert(c.target_object_id);
            }
        }
    };
    None
}

fn build_skip_cutscene_relay_connections<'a>(
    pickup_type: u32,
    obj_connections: &reader_writer::LazyArray<'a, structs::Connection>,
    scly_db: &HashMap<u32, (usize, structs::SclyObject<'a>)>,
) -> Vec<structs::Connection>
{
    let post_pickup_relay = search_for_scly_object(obj_connections, scly_db, |o| {
        o.property_data.as_relay()
            .map(|i| i.name.to_bytes() == b"Relay Post Pickup")
            .unwrap_or(false)
    }).unwrap();

    let mut connections = vec![];
    for conn in post_pickup_relay.connections.iter() {
        let connected_object = if let Some(obj) = scly_db.get(&conn.target_object_id) {
            &obj.1
        } else {
            connections.push(conn.into_owned());
            continue
        };
        if let Some(timer) = connected_object.property_data.as_timer() {
             let name = timer.name.to_bytes();
             if name == b"Timer Jingle" {
                 connections.extend(connected_object.connections.iter().map(|i| i.into_owned()));
             } else if name == b"Timer HUD" {
                 // We want to copy most of Timer HUD's connections, with a few exceptions
                 for conn in connected_object.connections.iter() {
                    let obj = if let Some(ref obj) = scly_db.get(&conn.target_object_id) {
                        &obj.1
                    } else {
                        connections.push(conn.into_owned());
                        continue
                    };

                    let is_log_screen_timer = obj.property_data.as_timer()
                        .map(|i| i.name.to_bytes() == &b"Timer - Delay Enter Logbook Screen"[..])
                        .unwrap_or(false);
                    // Skip player hints and a artifact log screen timers
                    // Note the special case for the Artifact of Truth's timer
                    if (is_log_screen_timer && obj.instance_id != 1049534) ||
                        obj.property_data.as_player_hint().is_some() {
                        continue
                    }
                    connections.push(conn.into_owned());
                 }
             } else {
                 connections.push(conn.into_owned());
             }
        } else if connected_object.property_data.as_player_hint().is_none() {
            // Skip the Player Hint objects.
            connections.push(conn.into_owned());
        }
    }

    // Stop here if not the Varia Suit
    if pickup_type != 22 {
        return connections
    }

    // We need a special case for the Varia Suit to unlock the doors
    let unlock_doors_relay = search_for_scly_object(obj_connections, scly_db, |o| {
        o.property_data.as_relay()
            .map(|i| i.name.to_bytes() == &b"!Relay Local End Suit Attainment Cinematic"[..])
            .unwrap_or(false)
    }).unwrap();

    for conn in unlock_doors_relay.connections.iter() {
        let connected_object = &scly_db.get(&conn.target_object_id).unwrap().1;
        if connected_object.property_data.as_dock().is_some() ||
           connected_object.property_data.as_trigger().is_some() {
            connections.push(conn.into_owned());
        }
    }

    connections
}

fn find_cutscene_trigger_relay<'a>(
    pickup_type: u32,
    obj_connections: &reader_writer::LazyArray<'a, structs::Connection>,
    scly_db: &HashMap<u32, (usize, structs::SclyObject<'a>)>,
) -> ScriptObjectLocation
{
    // We need to look for specific layer names depending on the pickup type. This is mostly the
    // result of the non-cutscene artifacts, for which the relay we're looking for is simply titled
    // "Relay".
    // We need this seperate static in order to get static lifetimes. Its kinda awful.
    static NAME_CANDIDATES: &'static [&'static [u8]] = &[
        b"!Relay Start Suit Attainment Cinematic",
        b"!Relay Local Start Suit Attainment Cinematic",
        b"Relay-start of cinema",
        b"Relay",
    ];
    let name_candidates: &[&[u8]] = match pickup_type {
        21 => &NAME_CANDIDATES[0..1],
        22 => &NAME_CANDIDATES[1..2],
        29 | 30 | 31 | 32 | 33 | 34 | 35 | 36 | 37 | 38 | 39 | 40
            => &NAME_CANDIDATES[2..4],
        _ => &NAME_CANDIDATES[2..3],
    };
    let obj = search_for_scly_object(obj_connections, scly_db, |o| {
        o.property_data.as_relay()
            .map(|i| name_candidates.contains(&i.name.to_bytes()))
            .unwrap_or(false)
    }).unwrap();
    ScriptObjectLocation {
        layer: scly_db[&obj.instance_id].0 as u32,
        instance_id: obj.instance_id,
    }
}

// We can get pretty close to the Claris's dependecies for each pickup, but some
// of them need custom modification to match exactly.
fn patch_dependencies(pickup_kind: u32, deps: &mut HashSet<ResourceKey>)
{
    // Don't ask me why; Claris seems to skip this one.
    deps.remove(&ResourceKey::new(0xA0DA476B, b"PART".into()));

    if pickup_kind == 19 {
        // Spiderball. I couldn't find any references to this outside of PAK resource
        // indexes and dependency lists.
        deps.insert(ResourceKey::new(0x00656374, b"CSKR".into()));
    } else if pickup_kind == 23 {
        // Phazon suit.
        deps.insert(ResourceKey::new(asset_ids::PHAZON_SUIT_SCAN, b"SCAN".into()));
        deps.insert(ResourceKey::new(asset_ids::PHAZON_SUIT_STRG, b"STRG".into()));

        // Remove the Gravity Suit's CMDL and ANCS
        deps.remove(&ResourceKey::new(asset_ids::GRAVITY_SUIT_CMDL, b"CMDL".into()));
        deps.remove(&ResourceKey::new(asset_ids::GRAVITY_SUIT_ANCS, b"ANCS".into()));
        deps.remove(&ResourceKey::new(0x08C625DA, b"TXTR".into()));
        deps.remove(&ResourceKey::new(0xA95D06BC, b"TXTR".into()));

        // Add the custom CMDL and textures
        deps.insert(ResourceKey::new(asset_ids::PHAZON_SUIT_CMDL, b"CMDL".into()));
        deps.insert(ResourceKey::new(asset_ids::PHAZON_SUIT_ANCS, b"ANCS".into()));
        deps.insert(ResourceKey::new(asset_ids::PHAZON_SUIT_TXTR1, b"TXTR".into()));
        deps.insert(ResourceKey::new(asset_ids::PHAZON_SUIT_TXTR2, b"TXTR".into()));
    };
}

fn create_nothing(pickup_table: &mut HashMap<usize, PickupData>)
{
    // Special case for Nothing
    let mut nothing_bytes = Vec::new();
    {
        let mut nothing_pickup = Reader::new(&pickup_table[&6].bytes)
                                        .read::<Pickup>(()).clone();
        nothing_pickup.name = Cow::Borrowed(CStr::from_bytes_with_nul(b"Nothing\0").unwrap());
        nothing_pickup.kind = 26; // This kind matches an energy refill
        nothing_pickup.max_increase = 0;
        nothing_pickup.curr_increase = 0;
        nothing_pickup.cmdl = asset_ids::NOTHING_CMDL;
        nothing_pickup.ancs.file_id = asset_ids::NOTHING_ANCS;
        nothing_pickup.actor_params.scan_params.scan = asset_ids::NOTHING_SCAN;
        nothing_pickup.write(&mut nothing_bytes).unwrap();
    }
    let mut nothing_deps: HashSet<_> = pickup_table[&6].deps.iter()
        .filter(|i| ![b"SCAN".into(), b"STRG".into(),
                      b"CMDL".into(), b"ANCS".into()].contains(&i.fourcc))
        .cloned()
        .collect();
    nothing_deps.remove(&ResourceKey::new(asset_ids::PHAZON_SUIT_TXTR1, b"TXTR".into()));
    nothing_deps.extend(&[
        ResourceKey::new(asset_ids::NOTHING_SCAN_STRG, b"STRG".into()),
        ResourceKey::new(asset_ids::NOTHING_SCAN, b"SCAN".into()),
        ResourceKey::new(asset_ids::NOTHING_CMDL, b"CMDL".into()),
        ResourceKey::new(asset_ids::NOTHING_ANCS, b"ANCS".into()),
        ResourceKey::new(asset_ids::NOTHING_TXTR, b"TXTR".into()),
    ]);
    let len = pickup_table.len();
    assert!(pickup_table.insert(len, PickupData {
        name: "Nothing",
        bytes: nothing_bytes,
        deps: nothing_deps,
        hudmemo_strg: asset_ids::NOTHING_ACQUIRED_HUDMEMO_STRG,
        // TODO replace with something silly or silence?
        attainment_audio_file_name: b"/audio/itm_x_short_02.dsp\0".to_vec(),
    }).is_none());
}

fn main()
{
    let file = File::open(args().nth(1).unwrap()).unwrap();
    let mmap = memmap::Mmap::open(&file, memmap::Protection::Read).unwrap();
    let mut reader = Reader::new(unsafe { mmap.as_slice() });
    let mut gc_disc: structs::GcDisc = reader.read(());

    let filenames = [
        "Metroid2.pak",
        "Metroid3.pak",
        "Metroid4.pak",
        "metroid5.pak",
        "Metroid6.pak",
    ];

    let mut i = 0;
    let mut pickup_table = HashMap::new();
    let mut cmdl_aabbs = HashMap::new();
    let mut locations = Vec::new();
    for f in &filenames {
        trace_pickup_deps(&mut gc_disc, f, &mut i, &mut pickup_table, &mut locations,
                          &mut cmdl_aabbs);
    }

    // Special case of Nothing and Phazon Suits' custom CMDLs
    let aabb = *cmdl_aabbs.get(&asset_ids::GRAVITY_SUIT_CMDL).unwrap();
    assert!(cmdl_aabbs.insert(asset_ids::PHAZON_SUIT_CMDL, aabb).is_none());
    assert!(cmdl_aabbs.insert(asset_ids::NOTHING_CMDL, aabb).is_none());

    create_nothing(&mut pickup_table);

    println!("// This file is generated by bin/resource_tracing.rs");
    println!("");
    println!("");

    println!("pub const PICKUP_LOCATIONS: &[(&str, &[RoomInfo]); 5] = &[");
    for (fname, locations) in filenames.iter().zip(locations.into_iter()) {
        // println!("    // {}", fname);
        println!("    ({:?}, &[", fname);
        for room_info in locations {
            println!("        RoomInfo {{");
            println!("            room_id: 0x{:08X},", room_info.room_id);
            println!("            name: {:?},", &room_info.name[..(room_info.name.len() - 1)]);
            println!("            pickup_locations: &[");
            for location in room_info.pickups {
                println!("                PickupLocation {{");
                println!("                    location: {:?},", location.location);
                println!("                    attainment_audio: {:?},", location.attainment_audio);
                println!("                    hudmemo: {:?},", location.hudmemo);
                if location.post_pickup_relay_connections.len() == 0 {
                    println!("                    post_pickup_relay_connections: &[]");
                } else {
                    println!("                    post_pickup_relay_connections: &[");
                    for conn in &location.post_pickup_relay_connections {
                        println!("                        Connection {{");
                        println!("                            state: {:?},", conn.state);
                        println!("                            message: {:?},", conn.message);
                        println!("                            target_object_id: 0x{:x},",
                                 conn.target_object_id);
                        println!("                        }},");
                    }
                    println!("                    ],");
                }
                println!("                }},");
            }
            println!("            ],");

            if room_info.objects_to_remove.len() == 0 {
                println!("            objects_to_remove: &[],");
            } else {
                println!("            objects_to_remove: &[");
                let mut objects_to_remove: Vec<_> = room_info.objects_to_remove.iter().collect();
                objects_to_remove.sort_by_key(|&(k, _)| k);
                for otr in objects_to_remove {
                    println!("                ObjectsToRemove {{");
                    println!("                    layer: {},", otr.0);
                    println!("                    instance_ids: &{:?},", otr.1);
                    println!("                }},");
                }
                println!("            ],");
            }
            println!("        }},");
        }
        println!("    ]),");
    }
    println!("];");

    println!("");
    println!("// Because const fns aren't powerful enough yet, we can't construct the actual");
    println!("// array at compile-time. So, instead, these are the building-blocks for that");
    println!("// actual array.");
    println!("const PICKUP_RAW_META: [PickupMetaRaw; {}] = [", pickup_table.len());
    const BYTES_PER_LINE: usize = 8;
    // Iterate using an explicit indexer so the output is sorted.
    for i in 0..pickup_table.len() {
        let ref pickup_data = pickup_table[&i];
        let pickup_bytes = &pickup_data.bytes;
        println!("    PickupMetaRaw {{");
        println!("        name: {:?},", pickup_data.name);
        println!("        pickup: &[");
        for y in 0..((pickup_bytes.len() + BYTES_PER_LINE - 1) / BYTES_PER_LINE) {
            let len = ::std::cmp::min(BYTES_PER_LINE, pickup_bytes.len() - y * BYTES_PER_LINE);
            print!("           ");
            for x in 0..len {
                print!(" 0x{:02X},", pickup_bytes[y * BYTES_PER_LINE + x]);
            }
            println!("");
        }
        println!("        ],");
        println!("        deps: &[");
        let mut deps: Vec<_> = pickup_data.deps.iter().collect();
        deps.sort();
        for dep in deps {
            println!("            (0x{:08X}, FourCC::from_bytes(b\"{}\")),", dep.file_id,
                                                                             dep.fourcc);
        }
        println!("        ],");
        println!("        hudmemo_strg: {:?},", pickup_data.hudmemo_strg);
        let filename = stdstr::from_utf8(&pickup_data.attainment_audio_file_name).unwrap();
        println!("        attainment_audio_file_name: {:?},", filename);
        println!("    }},");

    }
    println!("];");
    println!("");

    let mut cmdl_aabbs: Vec<_> = cmdl_aabbs.iter().collect();
    cmdl_aabbs.sort_by_key(|&(k, _)| k);
    println!("const PICKUP_CMDL_AABBS: [(u32, [u32; 6]); {}] = [", cmdl_aabbs.len());
    for (cmdl_id, aabb) in cmdl_aabbs {
        let aabb: [u32; 6] = unsafe { mem::transmute(*aabb) };
        println!("    (0x{:08X}, [0x{:08X}, 0x{:08X}, 0x{:08X}, 0x{:08X}, 0x{:08X}, 0x{:08X}]),",
                    cmdl_id, aabb[0], aabb[1], aabb[2], aabb[3], aabb[4], aabb[5]);
    }
    println!("];");
}
