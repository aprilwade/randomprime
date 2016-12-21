extern crate memmap;
extern crate configurer;

pub use configurer::*;

use reader_writer::{FourCC, Reader};

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::env::args;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Read};
use std::ffi::CString;

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

fn insert_deps<'a, I>(
    gc_disc: &mut structs::GcDisc<'a>,
    pak_name: &str,
    pickup_resources: &HashMap<(u32, FourCC), structs::Resource<'a>>,
    room_list: &'static [(u32, &'static [u8])],
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

        let pickup_layers = if let Some(&&(file_id, pickup_layers)) = room_list_iter.peek() {
            if file_id != curr_file_id {
                continue;
            }
            room_list_iter.next();
            pickup_layers
        } else {
            continue;
        };

        let mut area = editor.get_area(&mut cursor);

        let mut last_layer = 255;
        let mut nth_pickup = 0;
        for &layer_num in pickup_layers {
            if last_layer != layer_num {
                last_layer = layer_num;
                nth_pickup = 0;
            } else {
                nth_pickup += 1;
            }

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
            let ref mut layer = scly.layers.as_mut_vec()[layer_num as usize];
            let pickup = layer.objects.iter_mut()
                .filter(|obj| obj.property_data.object_type() == 0x11)
                .filter(|obj| {
                    let pickup = match obj.property_data {
                        structs::SclyProperty::Unknown { ref data, .. } => data.clone().read(()),
                        structs::SclyProperty::Pickup(ref pickup) => pickup.clone(),
                        _ => panic!(),
                    };
                    pickup.max_increase > 0
                })
                .nth(nth_pickup as usize)
                .unwrap();

            pickup.property_data.guess_kind();
            let pickup = match pickup.property_data {
                structs::SclyProperty::Pickup(ref mut pickup) => pickup,
                _ => panic!(),
            };
            let original_pickup = pickup.clone();
            *pickup = pickup_meta.pickup.clone();
            pickup.position = original_pickup.position;
            pickup.rotation = original_pickup.rotation;
            //pickup.scale = original_pickup.scale;
            pickup.hitbox = original_pickup.hitbox;
            pickup.scan_offset = original_pickup.scan_offset;
        }
    }
}

fn parse_pickup_layout<R: Read>(r: R)
    -> Vec<u8>
{
    let reader = BufReader::new(r);
    reader.lines().into_iter().map(|l| u8::from_str_radix(&l.unwrap(), 10).unwrap()).collect()
}


fn main()
{
    pickup_meta::setup_pickup_meta_table();

    let pickup_layout_file = File::open(args().nth(2).unwrap()).unwrap();
    let pickup_layout = parse_pickup_layout(pickup_layout_file);
    assert_eq!(pickup_layout.len(), 100);

    let file = File::open(args().nth(1).unwrap()).unwrap();
    let mmap = memmap::Mmap::open(&file, memmap::Protection::Read).unwrap();
    let mut reader = Reader::new(unsafe { mmap.as_slice() });
    let mut gc_disc: structs::GcDisc = reader.read(());

    let pickup_resources = collect_pickup_resources(&gc_disc);

    for pak_name in (&["Metroid2.pak"]).iter() {
        insert_deps(&mut gc_disc, pak_name,
                     &pickup_resources,
                     &mut pickup_meta::PICKUP_LOCATIONS[0],
                     &mut pickup_layout.iter().cloned().enumerate());
    }
    write_gc_disc(&mut gc_disc, &args().nth(3).unwrap());
}
