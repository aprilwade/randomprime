extern crate memmap;
extern crate clap;
extern crate configurer;

use clap::{Arg, App};

pub use configurer::*;

use reader_writer::{FourCC, Reader};
use reader_writer::generic_array::GenericArray;
use reader_writer::typenum::U3;
use reader_writer::num::{BigUint, Integer, ToPrimitive};

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::ffi::CString;
use std::io::Read;
use std::ascii::AsciiExt;

const METROID_PAK_NAMES: [&'static str; 5] = [
    "Metroid2.pak",
    "Metroid3.pak",
    "Metroid4.pak",
    "metroid5.pak",
    "Metroid6.pak",
];

fn write_gc_disc(gc_disc: &mut structs::GcDisc, path: &str)
{
    let out_iso = OpenOptions::new()
        .write(true)
        .create(true)
        .open(path)
        .unwrap();
    out_iso.set_len(structs::GC_DISC_LENGTH as u64).unwrap();

    gc_disc.write(&mut &out_iso);
}

fn collect_pickup_resources<'a>(gc_disc: &structs::GcDisc<'a>)
    -> HashMap<(u32, FourCC), structs::Resource<'a>>
{
    let mut looking_for: HashSet<_> = pickup_meta::pickup_meta_table().iter()
        .flat_map(|meta| meta.deps.iter().map(|key| *key))
        .collect();

    let mut found = HashMap::with_capacity(looking_for.len());

    for pak_name in METROID_PAK_NAMES.iter() {
        let file_entry = find_file(gc_disc, pak_name);
        let pak = match *file_entry.file().unwrap() {
            structs::FstEntryFile::Pak(ref pak) => Cow::Borrowed(pak),
            structs::FstEntryFile::Unknown(ref reader) => Cow::Owned(reader.clone().read(())),
            _ => panic!(),
        };

        for res in pak.resources.iter() {
            let key = (res.file_id, res.fourcc);
            if looking_for.remove(&key) {
                assert!(found.insert(key, res.clone()).is_none());
            }
        }
    }

    assert!(looking_for.is_empty());

    found
}

fn modify_pickups<'a, I>(
    gc_disc: &mut structs::GcDisc<'a>,
    pak_name: &str,
    pickup_resources: &HashMap<(u32, FourCC), structs::Resource<'a>>,
    room_list: &'static [(u32, &'static [pickup_meta::PickupLocation])],
    pickup_list_iter: &mut I,
)
    where I: Iterator<Item=(usize, u8)>,
{
    let file_entry = find_file_mut(gc_disc, pak_name);
    file_entry.guess_kind();
    let pak = match *file_entry.file_mut().unwrap() {
        structs::FstEntryFile::Pak(ref mut pak) => pak,
        _ => panic!(),
    };

    let resources = &mut pak.resources;
    let mut mlvl = resources.iter()
        .find(|i| i.fourcc == reader_writer::FourCC::from_bytes(b"MLVL"))
        .unwrap().clone();
    mlvl.guess_kind();
    let mlvl = match mlvl.kind {
        structs::ResourceKind::Mlvl(ref mut mlvl) => mlvl.clone(),
        _ => panic!(),
    };

    let mut editor = mlvl_wrapper::MlvlEditor::new(mlvl);

    let mut room_list_iter = room_list.iter().peekable();

    let mut cursor = resources.cursor();
    loop {
        let mut cursor = cursor.cursor_advancer();

        let curr_file_id = match cursor.peek().map(|res| (res.file_id, res.fourcc)) {
            None => break,
            Some((_, fourcc)) if fourcc == b"MLVL".into() => {
                // Update the Mlvl in the table with version we've been updating
                let mut res = cursor.value().unwrap();
                res.guess_kind();
                match res.kind {
                    structs::ResourceKind::Mlvl(ref mut mlvl_ref) => *mlvl_ref = editor.mlvl,
                    _ => panic!(),
                };
                break;
            },
            Some((_, fourcc)) if fourcc != b"MREA".into() => continue,
            Some((file_id, _)) => file_id,
        };

        let pickup_locations = if let Some(&&(file_id, pickup_locations)) = room_list_iter.peek() {
            if file_id != curr_file_id {
                continue;
            }
            room_list_iter.next();
            pickup_locations
        } else {
            continue;
        };

        let mut area = editor.get_area(&mut cursor);

        for &pickup_location in pickup_locations {

            let (i, pickup_num) = pickup_list_iter.next().unwrap();
            let ref pickup_meta = pickup_meta::pickup_meta_table()[pickup_num as usize];
            let iter = pickup_meta.deps.iter().map(|&(file_id, fourcc)| structs::Dependency {
                    asset_id: file_id,
                    asset_type: fourcc,
                });

            let name = CString::new(format!(
                    "Randomizer - Pickup {} ({:?})", i, pickup_meta.pickup.name)).unwrap();
            area.add_layer(name);

            let new_layer_idx = area.layer_flags.layer_count as usize - 1;
            area.add_dependencies(pickup_resources, new_layer_idx, iter);

            let scly = area.mrea().scly_section_mut();
            let layers = scly.layers.as_mut_vec();

            {
                let pickup = layers[pickup_location.location.layer as usize].objects.iter_mut()
                    .find(|obj| obj.instance_id ==  pickup_location.location.instance_id)
                    .unwrap();
                update_pickup(pickup, &pickup_meta);
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
    pickup.property_data.guess_kind();
    let pickup = match pickup.property_data {
        structs::SclyProperty::Pickup(ref mut pickup) => pickup,
        _ => panic!(),
    };
    let original_pickup = pickup.clone();

    let original_aabb = pickup_meta::aabb_for_pickup_cmdl(original_pickup.cmdl).unwrap();
    let new_aabb = pickup_meta::aabb_for_pickup_cmdl(pickup_meta.pickup.cmdl).unwrap();
    let original_center = calculate_center(original_aabb, original_pickup.rotation,
                                            original_pickup.scale);
    let new_center = calculate_center(new_aabb, pickup_meta.pickup.rotation,
                                        pickup_meta.pickup.scale);

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
    hudmemo.property_data.guess_kind();
    let hudmemo = match hudmemo.property_data {
        structs::SclyProperty::HudMemo(ref mut hudmemo) => hudmemo,
        _ => panic!(),
    };
    if let Some(strg) = pickup_meta.hudmemo_strg {
        hudmemo.strg = strg;
    }
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

fn main_inner() -> Result<(), String>
{
    pickup_meta::setup_pickup_meta_table();

    // TODO: Use base64 based pickup layout
    let matches = App::new("Metroid Prime Configuerer")
        .version("0.0")
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
        .get_matches();

    let input_iso_path = matches.value_of("input iso path").unwrap();
    let output_iso_path = matches.value_of("output iso path").unwrap();
    let pickup_layout = matches.value_of("pickup layout").unwrap();
    let skip_frigate = matches.is_present("skip frigate");

    let pickup_layout = parse_pickup_layout(pickup_layout)?;
    assert_eq!(pickup_layout.len(), 100);

    let file = File::open(input_iso_path).unwrap();
    let mmap = memmap::Mmap::open(&file, memmap::Protection::Read).unwrap();
    let mut reader = Reader::new(unsafe { mmap.as_slice() });
    let mut gc_disc: structs::GcDisc = reader.read(());

    let pickup_resources = collect_pickup_resources(&gc_disc);

    for (i, pak_name) in METROID_PAK_NAMES.iter().enumerate() {
        modify_pickups(&mut gc_disc, pak_name,
                       &pickup_resources,
                       &mut pickup_meta::PICKUP_LOCATIONS[i],
                       &mut pickup_layout.iter().cloned().enumerate());
    }

    if skip_frigate {
        patch_dol_skip_frigate(&mut gc_disc);
    }
    write_gc_disc(&mut gc_disc, output_iso_path);
    Ok(())
}

fn main()
{
    match main_inner() {
        Err(s) => println!("{}", s),
        Ok(()) => (),
    }
}
