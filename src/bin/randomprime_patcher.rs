extern crate memmap;
extern crate clap;
extern crate randomprime_patcher;

use clap::{Arg, App};
// XXX This is an undocumented enum
use clap::Format;

pub use randomprime_patcher::*;

use reader_writer::{FourCC, Reader, Writable};
use reader_writer::generic_array::GenericArray;
use reader_writer::typenum::U3;
use reader_writer::num::{BigUint, Integer, ToPrimitive};

use std::io;
use std::panic;
use std::ascii::AsciiExt;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::ffi::{CStr, CString};
use std::io::{Read, Write};

const METROID_PAK_NAMES: [&'static str; 5] = [
    "Metroid2.pak",
    "Metroid3.pak",
    "Metroid4.pak",
    "metroid5.pak",
    "Metroid6.pak",
];

const ARTIFACT_OF_TRUTH_REQ_LAYER: u32 = 24;
const ARTIFACT_TEMPLE_ID: u32 = 0x2398E906;

fn write_gc_disc(gc_disc: &mut structs::GcDisc, path: &str, mut pn: ProgressNotifier)
    -> Result<(), String>
{
    let out_iso = OpenOptions::new()
        .write(true)
        .create(true)
        .open(path)
        .map_err(|e| format!("Failed to open output file: {}", e))?;
    out_iso.set_len(structs::GC_DISC_LENGTH as u64)
        .map_err(|e| format!("Failed to open output file: {}", e))?;

    gc_disc.write(&mut &out_iso, &mut pn)
        .map_err(|e| format!("Error writing output file: {}", e))?;

    pn.notify_flushing_to_disk();
    Ok(())
}

// When changing a pickup, we need to give the room a copy of the resources/
// assests used by the pickup. Create a cache of all the resources needed by
// any pickup.
fn collect_pickup_resources<'a>(gc_disc: &structs::GcDisc<'a>)
    -> HashMap<(u32, FourCC), structs::Resource<'a>>
{
    let mut looking_for: HashSet<_> = pickup_meta::pickup_meta_table().iter()
        .flat_map(|meta| meta.deps.iter().map(|key| *key))
        .collect();

    let mut found = HashMap::with_capacity(looking_for.len());

    let extra_assets = pickup_meta::extra_assets();
    for res in extra_assets {
        looking_for.remove(&(res.file_id, res.fourcc()));
        assert!(found.insert((res.file_id, res.fourcc()), res.clone()).is_none());
    }

    for pak_name in METROID_PAK_NAMES.iter() {
        let file_entry = find_file(gc_disc, pak_name);
        let pak = match *file_entry.file().unwrap() {
            structs::FstEntryFile::Pak(ref pak) => Cow::Borrowed(pak),
            structs::FstEntryFile::Unknown(ref reader) => Cow::Owned(reader.clone().read(())),
            _ => panic!(),
        };


        for res in pak.resources.iter() {
            let key = (res.file_id, res.fourcc());
            if looking_for.remove(&key) {
                assert!(found.insert(key, res.clone()).is_none());
            }
        }
    }

    // Generate and add the assets for the Phazon Suit
    let (cmdl, ancs) = create_phazo_cmdl_and_ancs(&mut found);
    let key = (cmdl.file_id, cmdl.fourcc());
    if looking_for.remove(&key) {
        assert!(found.insert(key, cmdl).is_none());
    }
    let key = (ancs.file_id, ancs.fourcc());
    if looking_for.remove(&key) {
        assert!(found.insert(key, ancs).is_none());
    }

    assert!(looking_for.is_empty());

    found
}

fn create_phazo_cmdl_and_ancs<'a>(resources: &mut HashMap<(u32, FourCC), structs::Resource<'a>>)
    -> (structs::Resource<'a>, structs::Resource<'a>)
{
    let phazon_suit_cmdl = {
        let grav_suit_cmdl = ResourceData::new(&resources[&(0x95946E41, b"CMDL".into())]);
        let mut phazon_cmdl_bytes = grav_suit_cmdl.decompress().into_owned();

        // Ensure the length is a multiple of 32
        let len = phazon_cmdl_bytes.len();
        phazon_cmdl_bytes.extend(reader_writer::pad_bytes(32, len).iter());

        // Change which textures this points to
        0x50535431u32.write(&mut &mut phazon_cmdl_bytes[0x64..]).unwrap();
        0x50535432u32.write(&mut &mut phazon_cmdl_bytes[0x70..]).unwrap();
        pickup_meta::build_resource(
            0x50534D44,
            structs::ResourceKind::External(phazon_cmdl_bytes, b"CMDL".into())
        )
    };
    let phazon_suit_ancs = {
        let grav_suit_ancs = ResourceData::new(&resources[&(0x27A97006, b"ANCS".into())]);
        let mut phazon_ancs_bytes = grav_suit_ancs.decompress().into_owned();

        // Ensure the length is a multiple of 32
        let len = phazon_ancs_bytes.len();
        phazon_ancs_bytes.extend(reader_writer::pad_bytes(32, len).iter());

        // Change this to refer to the CMDL above
        0x50534D44u32.write(&mut &mut phazon_ancs_bytes[0x14..]).unwrap();
        pickup_meta::build_resource(
            0x5053414E,
            structs::ResourceKind::External(phazon_ancs_bytes, b"ANCS".into())
        )
    };
    (phazon_suit_cmdl, phazon_suit_ancs)
}

fn artifact_layer_change_template<'a>(instance_id: u32, pickup_kind: u32)
    -> structs::SclyObject<'a>
{
    let layer = if pickup_kind > 29 {
        pickup_kind - 28
    } else {
        assert!(pickup_kind == 29);
        ARTIFACT_OF_TRUTH_REQ_LAYER
    };
    structs::SclyObject {
        instance_id: instance_id,
        connections: reader_writer::LazyArray::Owned(Vec::new()),
        property_data: structs::SclyProperty::SpecialFunction(
            structs::SpecialFunction {
                name: Cow::Borrowed(CStr::from_bytes_with_nul(b"Artifact Layer Switch\0").unwrap()),
                position: GenericArray::map_slice(&[0., 0., 0.], Clone::clone),
                rotation: GenericArray::map_slice(&[0., 0., 0.], Clone::clone),
                type_: 16,
                // TODO Working around a compiler bug. Switch this back to being checked later.
                unknown0: Cow::Borrowed(unsafe { CStr::from_bytes_with_nul_unchecked(b"\0") }),
                unknown1: 0.,
                unknown2: 0.,
                unknown3: 0.,
                layer_change_room_id: 3442151074,
                layer_change_layer_id: layer,
                item_id: 0,
                unknown4: 1,
                unknown5: 0.,
                unknown6: 4294967295,
                unknown7: 4294967295,
                unknown8: 4294967295
            }
        ),
    }
}

fn post_pickup_relay_template<'a>(instance_id: u32, connections: &'static [structs::Connection])
    -> structs::SclyObject<'a>
{
    structs::SclyObject {
        instance_id: instance_id,
        connections: reader_writer::LazyArray::Owned(connections.to_owned()),
        property_data: structs::SclyProperty::Relay(structs::Relay {
            name: Cow::Owned(CString::new(b"Randomizer Post Pickup Relay".to_vec()).unwrap()),
            active: 1,
        })
    }
}

fn modify_pickups<'a, I, J>(
    gc_disc: &mut structs::GcDisc<'a>,
    pak_name: &str,
    pickup_resources: &HashMap<(u32, FourCC), structs::Resource<'a>>,
    room_list: &'static [pickup_meta::RoomInfo],
    pickup_list_iter: &mut I,
    fresh_instance_id_iter: &mut J,
)
    where I: Iterator<Item=(usize, &'static pickup_meta::PickupMeta)>,
          J: Iterator<Item=u32>,
{
    let file_entry = find_file_mut(gc_disc, pak_name);
    file_entry.guess_kind();
    let pak = match *file_entry.file_mut().unwrap() {
        structs::FstEntryFile::Pak(ref mut pak) => pak,
        _ => panic!(),
    };

    let resources = &mut pak.resources;

    // To appease the borrow checker, make a copy of the Mlvl on the stack that
    // we'll update as we go. When we're done manipulating all the other resources
    // in the pak, we'll write this copy over the one in the pak.
    let mlvl = resources.iter()
        .find(|i| i.fourcc() == reader_writer::FourCC::from_bytes(b"MLVL"))
        .unwrap().kind.as_mlvl().unwrap().into_owned();

    let mut editor = mlvl_wrapper::MlvlEditor::new(mlvl);

    let mut room_list_iter = room_list.iter().peekable();

    let mut cursor = resources.cursor();
    loop {
        let mut cursor = cursor.cursor_advancer();

        let curr_file_id = match cursor.peek().map(|res| (res.file_id, res.fourcc())) {
            None => break,
            Some((_, fourcc)) if fourcc == b"MLVL".into() => {
                // Update the Mlvl in the table with version we've been updating
                let mut res = cursor.value().unwrap().kind.as_mlvl_mut().unwrap();
                *res = editor.mlvl;
                // The Mlvl is the last entry in the PAK, so break here.
                break;
            },
            Some((_, fourcc)) if fourcc == b"SAVW".into() && pak_name == "metroid5.pak" => {
                // Add a scan for the Phazon suit.
                let mut savw = cursor.value().unwrap().kind.as_savw_mut().unwrap();
                savw.scan_array.as_mut_vec().push(structs::ScannableObject {
                    scan: 0x50535343,
                    logbook_category: 0,
                });
                continue
            },
            Some((file_id, fourcc)) if fourcc == b"MREA".into() => file_id,
            _ => continue,
        };

        // The default case is MREA, since its the most complex by far.
        let (pickup_locations, removals) = if let Some(&&room_info) = room_list_iter.peek() {
            if room_info.room_id != curr_file_id {
                continue;
            }
            room_list_iter.next();
            (room_info.pickup_locations, room_info.objects_to_remove)
        } else {
            continue;
        };

        let mut area = editor.get_area(&mut cursor);

        // Remove objects
        {
            let scly = area.mrea().scly_section_mut();
            let layers = scly.layers.as_mut_vec();
            for otr in removals {
                layers[otr.layer as usize].objects.as_mut_vec()
                    .retain(|i| !otr.instance_ids.contains(&i.instance_id));
            }
        }

        for &pickup_location in pickup_locations {

            let (i, pickup_meta) = pickup_list_iter.next().unwrap();
            let iter = pickup_meta.deps.iter().map(|&(file_id, fourcc)| structs::Dependency {
                    asset_id: file_id,
                    asset_type: fourcc,
                });

            // TODO: Re-randomization: reuse the same layer, just clear its contents
            //       (and remove any connections to any objects contained within it)
            let name = CString::new(format!(
                    "Randomizer - Pickup {} ({:?})", i, pickup_meta.pickup.name)).unwrap();
            area.add_layer(name);

            let new_layer_idx = area.layer_flags.layer_count as usize - 1;
            area.add_dependencies(pickup_resources, new_layer_idx, iter);

            if curr_file_id == ARTIFACT_TEMPLE_ID {
                // If this room is the Artifact Temple, patch it.
                assert_eq!(pickup_locations.len(), 1, "Sanity check");
                fix_artifact_of_truth_requirement(&mut area, pickup_meta.pickup.kind,
                                                  fresh_instance_id_iter);
            }

            let scly = area.mrea().scly_section_mut();
            let layers = scly.layers.as_mut_vec();

            let mut additional_connections = Vec::new();

            // Add a post-pickup relay. This is used to support cutscene-skipping
            let instance_id = fresh_instance_id_iter.next().unwrap();
            let relay = post_pickup_relay_template(instance_id,
                                                   pickup_location.post_pickup_relay_connections);
            layers[new_layer_idx].objects.as_mut_vec().push(relay);
            additional_connections.push(structs::Connection {
                state: 1,
                message: 13,
                target_object_id: instance_id,
            });

            // If this is an artifact, insert a layer change function
            let pickup_kind = pickup_meta.pickup.kind;
            if pickup_kind >= 29 && pickup_kind <= 40 {
                let instance_id = fresh_instance_id_iter.next().unwrap();
                let function = artifact_layer_change_template(instance_id, pickup_kind);
                layers[new_layer_idx].objects.as_mut_vec().push(function);
                additional_connections.push(structs::Connection {
                    state: 1,
                    message: 7,
                    target_object_id: instance_id,
                });
            }

            {
                let pickup = layers[pickup_location.location.layer as usize].objects.iter_mut()
                    .find(|obj| obj.instance_id ==  pickup_location.location.instance_id)
                    .unwrap();
                update_pickup(pickup, &pickup_meta);
                if additional_connections.len() > 0 {
                    pickup.connections.as_mut_vec().extend_from_slice(&additional_connections);
                }
            }
            if let Some(ref hudmemo) = pickup_location.hudmemo {
                let hudmemo = layers[hudmemo.layer as usize].objects.iter_mut()
                    .find(|obj| obj.instance_id ==  hudmemo.instance_id)
                    .unwrap();
                update_hudmemo(hudmemo, &pickup_meta);
            }
        }
    }
}

fn update_pickup(pickup: &mut structs::SclyObject, pickup_meta: &pickup_meta::PickupMeta)
{
    let pickup = pickup.property_data.as_pickup_mut().unwrap();
    let original_pickup = pickup.clone();

    let original_aabb = pickup_meta::aabb_for_pickup_cmdl(original_pickup.cmdl).unwrap();
    let new_aabb = pickup_meta::aabb_for_pickup_cmdl(pickup_meta.pickup.cmdl).unwrap();
    let original_center = calculate_center(original_aabb, original_pickup.rotation,
                                            original_pickup.scale);
    let new_center = calculate_center(new_aabb, pickup_meta.pickup.rotation,
                                        pickup_meta.pickup.scale);

    // The pickup needs to be repositioned so that the center of its model
    // matches the center of the original.
    *pickup = structs::Pickup {
        position: GenericArray::from_slice(&[
            original_pickup.position[0] - (new_center[0] - original_center[0]),
            original_pickup.position[1] - (new_center[1] - original_center[1]),
            original_pickup.position[2] - (new_center[2] - original_center[2]),
        ]),
        rotation: original_pickup.rotation,
        hitbox: original_pickup.hitbox,
        scan_offset: GenericArray::from_slice(&[
            original_pickup.scan_offset[0] + (new_center[0] - original_center[0]),
            original_pickup.scan_offset[1] + (new_center[1] - original_center[1]),
            original_pickup.scan_offset[2] + (new_center[2] - original_center[2]),
        ]),

        fade_in_timer: original_pickup.fade_in_timer,
        unknown: original_pickup.unknown,
        active: original_pickup.active,

        ..(pickup_meta.pickup.clone())
    };
}

fn update_hudmemo(hudmemo: &mut structs::SclyObject, pickup_meta: &pickup_meta::PickupMeta)
{
    let hudmemo = hudmemo.property_data.as_hud_memo_mut().unwrap();
    hudmemo.strg = pickup_meta.hudmemo_strg;
    hudmemo.first_message_timer = 1.;
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
        let original = coordinate.clone();
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

fn parse_pickup_layout(text: &str) -> Result<Vec<u8>, String>
{
    const LAYOUT_CHAR_TABLE: [u8; 64] =
        *b"ABCDEFGHIJKLMNOPQRSTUWVXYZabcdefghijklmnopqrstuwvxyz0123456789-_";

    if !text.is_ascii() {
        return Err("Pickup layout string contains non-ascii characters.".to_string());
    }
    let text = text.as_bytes();
    if text.len() != 87 {
        return Err("Pickup layout should be exactly 87 characters".to_string());
    }

    let mut sum: BigUint = 0u8.into();
    let mut res = vec![];
    for c in text.iter().rev() {
        if let Some(idx) = LAYOUT_CHAR_TABLE.iter().position(|i| i == c) {
            sum = sum * BigUint::from(64u8) + BigUint::from(idx);
        } else {
            return Err(format!("Pickup layout contains invalid character '{}'.", c));
        }
    }

    // Reverse the order of the odd bits
    let mut bits = sum.to_str_radix(2).into_bytes();
    for i in 0..(bits.len() / 4) {
        let len = bits.len();
        bits.swap(i * 2 + 1, len - i * 2 - 1);
    }
    sum = BigUint::parse_bytes(&bits, 2).unwrap();

    // The upper 5 bits are a checksum, so seperate them from the sum.
    let checksum_bitmask = BigUint::from(0b11111u8) << 517;
    let checksum = sum.clone() & checksum_bitmask;
    sum = sum - checksum.clone();
    let checksum = (checksum >> 517).to_u8().unwrap();

    let mut computed_checksum = 0;
    {
        let mut sum = sum.clone();
        while sum > 0u8.into() {
            let remainder = (sum.clone() & BigUint::from(0b11111u8)).to_u8().unwrap();
            computed_checksum = (computed_checksum + remainder) % 32;
            sum = sum >> 5;
        }
    }
    if checksum != computed_checksum {
        return Err("Pickup layout checksum failed.".to_string());
    }

    for _ in 0..100 {
        let (quotient, remainder) = sum.div_rem(&36u8.into());
        res.push(remainder.to_u8().unwrap());
        sum = quotient;
    }

    assert!(sum == 0u8.into());

    res.reverse();
    Ok(res)
}

// Patches the current room to make the Artifact of Truth required to complete
// the game. This logic is based on the observed behavior of Claris's randomizer.
// XXX I still don't entirely understand why there needs to be a special case
//     if an artifact is placed in this room.
fn fix_artifact_of_truth_requirement<I>(area: &mut mlvl_wrapper::MlvlArea,
                                        pickup_kind: u32,
                                        fresh_instance_id_iter: &mut I)
    where I: Iterator<Item=u32>,
{
    let truth_req_layer_id = area.layer_flags.layer_count;
    assert_eq!(truth_req_layer_id, ARTIFACT_OF_TRUTH_REQ_LAYER);

    // Create a new layer that will be toggled on when the Artifact of Truth is collected
    area.add_layer(CString::new("Randomizer - Got Artifact 1".to_string()).unwrap());

    // TODO: Manually verify the correct layers are being toggled
    if pickup_kind != 29 {
        // If the item in the Artifact Temple isn't the artifact of truth, mark
        // the new layer inactive. Note, the layer is active when created.
        area.layer_flags.flags &= !(1 << truth_req_layer_id);
    }
    if pickup_kind >= 30 && pickup_kind <= 40 {
        // If the item in the Artfact Temple is an artifact (other than Truth)
        // mark its layer as active.
        area.layer_flags.flags |= 1 << (pickup_kind - 28);
    }
    // TODO: Re-randomizing: the other Got Artifact layers need to be marked inactive

    let scly = area.mrea().scly_section_mut();

    // A relay is created and connected to "Relay Show Progress 1"
    let new_relay_instance_id = fresh_instance_id_iter.next().unwrap();
    let new_relay = structs::SclyObject {
        instance_id: new_relay_instance_id,
        connections: reader_writer::LazyArray::Owned(vec![
            structs::Connection {
                state: 9,
                message: 13,
                target_object_id: 1048869,
            },
        ]),
        property_data: structs::SclyProperty::Relay(structs::Relay {
            name: Cow::Borrowed(CStr::from_bytes_with_nul(b"Relay Show Progress1\0").unwrap()),
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
        state: 9,
        message: 13,
        target_object_id: new_relay_instance_id,
    });
}

fn patch_starting_pickups<'a>(gc_disc: &mut structs::GcDisc<'a>, mut starting_items: u64)
{
    let file_entry = find_file_mut(gc_disc, "Metroid4.pak");
    file_entry.guess_kind();
    let pak = match *file_entry.file_mut().unwrap() {
        structs::FstEntryFile::Pak(ref mut pak) => pak,
        _ => panic!(),
    };


    // Find the first MREA in the pak
    let mut cursor = pak.resources.cursor();
    loop {
        if cursor.peek().unwrap().fourcc() == b"MREA".into() {
            break;
        }
        cursor.next();
    }
    let mrea = cursor.value().unwrap().kind.as_mrea_mut().unwrap();
    let scly = mrea.scly_section_mut();

    let mut fetch_bits = |bits: u8| {
        let ret = starting_items & ((1 << bits) - 1);
        starting_items = starting_items >> bits;
        ret as u32
    };

    // The object we want is in the first layer.
    let ref mut layer = scly.layers.as_mut_vec()[0];
    let obj = layer.objects.iter_mut().find(|obj| obj.property_data.object_type() == 0xF).unwrap();
    let spawn_point = obj.property_data.as_spawn_point_mut().unwrap();

    println!("Starting pickups set:");

    spawn_point.missiles = fetch_bits(8);
    println!("    missiles: {}", spawn_point.missiles);

    spawn_point.energy_tanks = fetch_bits(4);
    println!("    energy_tanks: {}", spawn_point.energy_tanks);

    spawn_point.power_bombs = fetch_bits(3);
    println!("    power_bombs: {}", spawn_point.power_bombs);

    spawn_point.wave = fetch_bits(1);
    println!("    wave: {}", spawn_point.wave);

    spawn_point.ice = fetch_bits(1);
    println!("    ice: {}", spawn_point.ice);

    spawn_point.plasma = fetch_bits(1);
    println!("    plasma: {}", spawn_point.plasma);

    spawn_point.charge = fetch_bits(1);
    println!("    charge: {}", spawn_point.plasma);

    spawn_point.morph_ball = fetch_bits(1);
    println!("    morph_ball: {}", spawn_point.morph_ball);

    spawn_point.bombs = fetch_bits(1);
    println!("    bombs: {}", spawn_point.bombs);

    spawn_point.spider_ball = fetch_bits(1);
    println!("    spider_ball: {}", spawn_point.spider_ball);

    spawn_point.boost_ball = fetch_bits(1);
    println!("    boost_ball: {}", spawn_point.boost_ball);

    spawn_point.gravity_suit = fetch_bits(1);
    println!("    gravity_suit: {}", spawn_point.gravity_suit);

    spawn_point.phazon_suit = fetch_bits(1);
    println!("    phazon_suit: {}", spawn_point.phazon_suit);

    spawn_point.thermal_visor = fetch_bits(1);
    println!("    thermal_visor: {}", spawn_point.thermal_visor);

    spawn_point.xray= fetch_bits(1);
    println!("    xray: {}", spawn_point.xray);

    spawn_point.space_jump = fetch_bits(1);
    println!("    space_jump: {}", spawn_point.space_jump);

    spawn_point.grapple = fetch_bits(1);
    println!("    grapple: {}", spawn_point.grapple);

    spawn_point.super_missile = fetch_bits(1);
    println!("    super_missile: {}", spawn_point.super_missile);

}

fn patch_dol_skip_frigate<'a>(gc_disc: &mut structs::GcDisc<'a>)
{
    let dol = find_file_mut(gc_disc, "default.dol");
    let file = dol.file_mut().unwrap();
    let reader = match file {
        &mut structs::FstEntryFile::Unknown(ref reader) => reader.clone(),
        _ => panic!(),
    };

    // Replace 4 of the bytes in the main dol. By using chain() like this, we
    // can avoid copying the contents of the dol onto the heap.
    static REPLACEMENT_1: &'static [u8] = &[0x39, 0xF3];
    static REPLACEMENT_2: &'static [u8] = &[0xDE, 0x28];
    let data = reader[..0x1FF1E]
        .chain(REPLACEMENT_1)
        .chain(&reader[0x1FF20..0x1FF2A])
        .chain(REPLACEMENT_2)
        .chain(&reader[0x1FF2C..]);
    *file = structs::FstEntryFile::ExternalFile(structs::ReadWrapper::new(data), reader.len());
}

struct ProgressNotifier
{
    total_size: usize,
    bytes_so_far: usize,
    quiet: bool,
}

impl ProgressNotifier
{
    fn new(quiet: bool) -> ProgressNotifier
    {
        ProgressNotifier {
            total_size: 0,
            bytes_so_far: 0,
            quiet: quiet,
        }
    }

    fn notify_flushing_to_disk(&mut self)
    {
        if self.quiet {
            return;
        }
        println!("Flushing written data to the disk...");
    }
}

impl structs::ProgressNotifier for ProgressNotifier
{
    fn notify_total_bytes(&mut self, total_size: usize)
    {
        self.total_size = total_size
    }

    fn notify_writing_file(&mut self, file_name: &reader_writer::CStr, file_bytes: usize)
    {
        if self.quiet {
            return;
        }
        let percent = self.bytes_so_far as f64 / self.total_size as f64 * 100.;
        println!("{:02.0}% -- Writing file {:?}", percent, file_name);
        self.bytes_so_far += file_bytes;
    }

    fn notify_writing_header(&mut self)
    {
        if self.quiet {
            return;
        }
        let percent = self.bytes_so_far as f64 / self.total_size as f64 * 100.;
        println!("{:02.0}% -- Writing ISO header", percent);
    }
}

fn main_inner() -> Result<(), String>
{
    pickup_meta::setup_pickup_meta_table();

    let matches = App::new("randomprime ISO patcher")
        .version("0.1.0")
        .arg(Arg::with_name("input iso path")
            .long("input-iso")
            .required(true)
            .takes_value(true))
        .arg(Arg::with_name("output iso path")
            .long("output-iso")
            .required(true)
            .takes_value(true))
        .arg(Arg::with_name("pickup layout")
            .long("layout")
            .required(true)
            .takes_value(true))
        .arg(Arg::with_name("skip frigate")
            .long("skip-frigate"))
        .arg(Arg::with_name("quiet")
            .long("quiet"))
        .arg(Arg::with_name("change starting items")
            .long("starting-items")
            .hidden(true)
            .takes_value(true)
            .validator(|s| s.parse::<u64>().map(|_| ())
                                           .map_err(|_| "Expected an integer".to_string())))
        .get_matches();

    let input_iso_path = matches.value_of("input iso path").unwrap();
    let output_iso_path = matches.value_of("output iso path").unwrap();
    let pickup_layout = matches.value_of("pickup layout").unwrap();
    let skip_frigate = matches.is_present("skip frigate");
    let quiet = matches.is_present("quiet");
    let starting_items = matches.value_of("change starting items");

    let pickup_layout = parse_pickup_layout(pickup_layout)?;
    assert_eq!(pickup_layout.len(), 100);

    let file = File::open(input_iso_path)
                .map_err(|e| format!("Failed to open input iso: {}", e))?;
    let mmap = memmap::Mmap::open(&file, memmap::Protection::Read)
                .map_err(|e| format!("Failed to open input iso: {}", e))?;
    let mut reader = Reader::new(unsafe { mmap.as_slice() });

    // On non-debug builds, suppress the default panic message and print a more helpful and
    // user-friendly one
    if !cfg!(debug_assertions) {
        panic::set_hook(Box::new(|_| {
            let _ = writeln!(io::stderr(), "{} \
An error occurred while parsing the input ISO. \
This most likely means your ISO is corrupt. \
Please verify that your ISO matches one of the following hashes:
MD5:  737cbfe7230af3df047323a3185d7e57
SHA1: 1c8b27af7eed2d52e7f038ae41bb682c4f9d09b5
", Format::Error("error:"));
        }));
    }

    let mut gc_disc: structs::GcDisc = reader.read(());

    if &gc_disc.header.game_identifier() != b"GM8E01" {
        Err("The input ISO doesn't appear to be Metroid Prime.".to_string())?
    }

    let pickup_resources = collect_pickup_resources(&gc_disc);

    let mut layout_iter = pickup_layout.iter()
        .map(|n| &pickup_meta::pickup_meta_table()[*n as usize])
        .enumerate();
    let mut fresh_instance_id_range = 0xDEEF0000..;
    for (i, pak_name) in METROID_PAK_NAMES.iter().enumerate() {
        modify_pickups(&mut gc_disc, pak_name,
                       &pickup_resources,
                       &mut pickup_meta::PICKUP_LOCATIONS[i],
                       &mut layout_iter,
                       &mut fresh_instance_id_range);
    }

    if skip_frigate {
        patch_dol_skip_frigate(&mut gc_disc);

        // To reduce the amount of data that needs to be copied, empty the contents of the pak
        let file_entry = find_file_mut(&mut gc_disc, "Metroid1.pak");
        file_entry.guess_kind();
        match file_entry.file_mut() {
            Some(&mut structs::FstEntryFile::Pak(ref mut pak)) => pak.resources.clear(),
            _ => (),
        };
    }

    if let Some(starting_items) = starting_items.map(|s| s.parse::<u64>().unwrap()) {
        patch_starting_pickups(&mut gc_disc, starting_items);
    }

    let pn = ProgressNotifier::new(quiet);
    write_gc_disc(&mut gc_disc, output_iso_path, pn)?;
    println!("Done");
    Ok(())
}

fn main()
{
    let _ = match main_inner() {
        Err(s) => writeln!(io::stderr(), "{} {}", Format::Error("error:"), s),
        Ok(()) => Ok(()),
    };
}
