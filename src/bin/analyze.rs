#![allow(dead_code, unused_imports)]

extern crate memmap;
// extern crate crypto;
extern crate flate2;
extern crate randomprime;
extern crate sha2;

pub use randomprime::*;

// use crypto::digest::Digest;

use randomprime::elevators::*;
use reader_writer::byteorder::{LittleEndian, ReadBytesExt};
use reader_writer::{Readable, Reader};

use std::collections::{HashMap, HashSet};
use std::env::args;
use std::fs::File;
use std::io::{self, Write};

fn dump_ciso()
{
    let file = File::open(args().nth(1).unwrap()).unwrap();
    let mmap = memmap::Mmap::open(&file, memmap::Protection::Read).unwrap();
    // let mut reader = Reader::new(unsafe { mmap.as_slice() });
    let slice = unsafe { mmap.as_slice() };

    use sha2::{Digest, Sha512};
    let mut hasher = Sha512::default();
    hasher.input(&slice[0x8000..]);
    println!(
        "{}",
        hasher
            .result()
            .iter()
            .flat_map(|b| format!("{:x}", b).chars().collect::<Vec<char>>())
            .collect::<String>()
    );
    panic!();
}

fn dump_gcz()
{
    let file = File::open(args().nth(1).unwrap()).unwrap();
    let mmap = memmap::Mmap::open(&file, memmap::Protection::Read).unwrap();
    // let mut reader = Reader::new(unsafe { mmap.as_slice() });
    let mut slice = unsafe { mmap.as_slice() };

    assert_eq!(slice.read_u32::<LittleEndian>().unwrap(), 0xB10BC001);
    assert_eq!(slice.read_u32::<LittleEndian>().unwrap(), 0);
    let _compressed_size = slice.read_u64::<LittleEndian>().unwrap();
    let _decompressed_size = slice.read_u64::<LittleEndian>().unwrap();
    let _block_size = slice.read_u32::<LittleEndian>().unwrap();
    let block_count = slice.read_u32::<LittleEndian>().unwrap();

    let offsets: Vec<u64> = (0..block_count)
        .map(|_| slice.read_u64::<LittleEndian>().unwrap())
        .collect();
    let _hashes: Vec<u32> = (0..block_count)
        .map(|_| slice.read_u32::<LittleEndian>().unwrap())
        .collect();

    let len0 = offsets[1] as usize;

    // TODO: Try decompressing block 0
    let mut output = vec![0; 16 * 1024];
    let mut decompressor = flate2::Decompress::new(true);
    println!(
        "{:?}",
        decompressor
            .decompress(&slice[..len0], &mut output, flate2::FlushDecompress::Finish)
            .unwrap()
    );
    println!("{}", decompressor.total_out());
    panic!(
        "{:?}",
        output[0..10]
            .iter()
            .map(|i| format!("{:x}", i))
            .collect::<Vec<_>>()
    );
}

fn print_fst(gc_disc: &structs::GcDisc)
{
    println!("{:#?}", gc_disc.header);
    let fst = &gc_disc.file_system_table;
    let mut entries: Vec<_> = fst
        .fst_entries
        .iter()
        // .filter(|e| !e.is_folder())
        .collect();
    entries.sort_by(|l, r| l.offset.cmp(&r.offset).reverse());
    for entry in entries.iter() {
        println!(
            "{:?} : {} {} {}",
            entry.name, entry.offset, entry.length, entry.name_offset
        );
    }
}

fn summarize_resources(gc_disc: &mut structs::GcDisc, pak_name: &str)
{
    let pak = match *gc_disc.find_file(pak_name).file().unwrap() {
        structs::FstEntryFile::Pak(ref pak) => pak.clone(),
        structs::FstEntryFile::Unknown(ref reader) => reader.clone().read(()),
        _ => unreachable!(),
    };
    println!("Resource Count {}", pak.resources.len());
    for res in pak.resources.iter() {
        /*if res.file_id == 0xB402D72C {
            let mut reader = match res.kind {
                structs::ResourceKind::Unknown(ref reader) => reader.clone(),
                _ => panic!(),
            };
            let size: u32 = reader.read(());
            let header: u16 = reader.read(());
            let mut output = vec![0; size as usize];
            flate2::Decompress::new(false).decompress(&reader, &mut output, flate2::Flush::Finish).unwrap();
            let mut outfile = OpenOptions::new()
                .write(true)
                .create(true)
                .open("/Users/twade/workspace/prime_randomizer/rust/part")
                .unwrap();
            use std::io::Write;
            outfile.write_all(&output);
        }*/
        /* let mut md5 = crypto::md5::Md5::new();
        match res.kind {
            structs::ResourceKind::Unknown(ref reader, _) => md5.input(&reader),
            _ => panic!(),
        };*/
        println!(
            "{:08X}.{}: {} - {} {}",
            res.file_id,
            res.fourcc(),
            /* md5.result_str()*/ "",
            res.compressed,
            res.size()
        );
    }
}

fn summarize_diff_mreas(gc_disc: &mut structs::GcDisc, pak_name: &str)
{
    let pak = match *gc_disc.find_file(pak_name).file().unwrap() {
        structs::FstEntryFile::Pak(ref pak) => pak.clone(),
        structs::FstEntryFile::Unknown(ref reader) => reader.clone().read(()),
        _ => unreachable!(),
    };
    for res in pak.resources.iter() {
        if res.fourcc() != reader_writer::FourCC::from_bytes(b"MLVL") {
            continue;
        };

        let mlvl = res.kind.as_mlvl().unwrap().into_owned();
        println!("{:#?}", mlvl);
        /*if res.fourcc != reader_writer::FourCC::from_bytes(b"MREA") {
            continue;
        };

        let mut md5 = crypto::md5::Md5::new();
        match res.kind {
            structs::ResourceKind::Unknown(ref reader) => md5.input(&reader),
            _ => panic!(),
        };
        println!("{:08X}.{}: {}", res.file_id, res.fourcc, md5.result_str());

        let mut res = res.clone();
        res.guess_kind();

        let mrea = match res.kind {
            structs::ResourceKind::Mrea(ref mrea) => mrea,
            _ => panic!(),
        };

        let s = mrea.sections.linear_get(mrea.scly_section_idx as usize).unwrap();
        let scly: structs::Scly = match *s {
            structs::MreaSection::Unknown(ref reader) => reader.clone().read(()),
            _ => panic!(),
        };
        // TODO: Layer names!
        for (i, layer) in scly.layers.iter().enumerate() {
            println!("  Layer {}, {} objects", i, layer.objects.len());
        }
        /*println!("  SCLY section idx: {}", mrea.scly_section_idx);
        for (i, section) in mrea.sections.iter().enumerate() {
            let mut md5 = crypto::md5::Md5::new();
            match *section {
                structs::MreaSection::Unknown(ref reader) => md5.input(&reader),
                _ => panic!(),
            };
            println!("  {}: {}", i, md5.result_str());
        }*/
        */
    }
}

fn dump_scly_data(gc_disc: &mut structs::GcDisc, pak_name: &str)
{
    let file_entry = gc_disc.find_file(pak_name);
    let pak = match *file_entry.file().unwrap() {
        structs::FstEntryFile::Pak(ref pak) => pak.clone(),
        structs::FstEntryFile::Unknown(ref reader) => reader.clone().read(()),
        _ => panic!(),
    };

    let mut resources = pak.resources;
    let mlvl = resources
        .iter()
        .find(|i| i.fourcc() == reader_writer::FourCC::from_bytes(b"MLVL"))
        .unwrap()
        .kind
        .as_mlvl()
        .unwrap()
        .into_owned();

    println!("{:#?}", mlvl.memory_relays);

    let mut editor = mlvl_wrapper::MlvlEditor::new(mlvl);

    //for res in resources.iter() {
    let mut cursor = resources.cursor();
    loop {
        let mut cursor = cursor.cursor_advancer();

        let res = if let Some(res) = cursor.peek() {
            res.into_owned()
        } else {
            break;
        };
        if res.fourcc() != reader_writer::FourCC::from_bytes(b"MREA") {
            continue;
        };

        let mut area = editor.get_area(&mut cursor);

        println!(
            "{:x}.MREA - {} {:b}",
            res.file_id, area.layer_flags.layer_count, area.layer_flags.flags
        );
        /*use std::iter::once;
        use std::borrow::Cow;
        use std::ffi::CStr;
        let n = Cow::Borrowed(CStr::from_bytes_with_nul(b"Others\0").unwrap());
        let iter = area.layer_names.iter().chain(once(&n)).zip(area.mlvl_area.dependencies.deps.iter());
        for (layer_name, deps) in iter {
            println!("  {:?} {}", layer_name, deps.len());
            println!("    [");
            for d in deps.iter() {
                println!("      ({:08X}.{}),", d.asset_id, d.asset_type);
            }
            println!("    ]");
        }*/

        let scly = area.mrea().scly_section_mut().clone();

        assert_eq!(scly.layers.len(), area.layer_names.len());
        for (scly_layer, name) in scly.layers.iter().zip(area.layer_names.iter()) {
            println!("  {:?}: {}", name, scly_layer.objects.len());
            for obj in scly_layer.objects.iter() {
                let mut obj = obj.into_owned();
                obj.property_data.guess_kind();
                //println!("{} {:#?}", obj.instance_id, obj.property_data);
                //println!("{:#?}", obj.connections);
                /* let print = match obj.property_data {
                    structs::SclyProperty::Unknown { .. } => false,
                    _ => true,
                };*/
                if let structs::SclyProperty::Unknown { ref data, .. } = obj.property_data {
                    let data = data.offset(4);
                    use std::ffi::CStr;
                    println!(
                        "Unknown {} -- {:x} {:?}",
                        obj.instance_id,
                        obj.property_data.object_type(),
                        data.iter()
                            .enumerate()
                            .find(|(_i, b)| **b == 0)
                            .and_then(|(i, _)| CStr::from_bytes_with_nul(&data[..i + 1]).ok())
                    );
                } else {
                    println!("{} {:#?}", obj.instance_id, obj.property_data);
                }
                println!("{:#?}", obj.connections);
            }
        }
    }
}

fn trace_object_connections(
    gc_disc: &mut structs::GcDisc,
    pak_name: &str,
    room_id: u32,
    instance_id: u32,
)
{
    let file_entry = gc_disc.find_file(pak_name);
    let pak = match *file_entry.file().unwrap() {
        structs::FstEntryFile::Pak(ref pak) => pak.clone(),
        structs::FstEntryFile::Unknown(ref reader) => reader.clone().read(()),
        _ => panic!(),
    };

    let mut resources = pak.resources;
    let mlvl = resources
        .iter()
        .find(|i| i.fourcc() == reader_writer::FourCC::from_bytes(b"MLVL"))
        .unwrap()
        .kind
        .as_mlvl()
        .unwrap()
        .into_owned();

    let mut editor = mlvl_wrapper::MlvlEditor::new(mlvl);

    let mut object_map = HashMap::new();

    //for res in resources.iter() {
    let mut cursor = resources.cursor();
    loop {
        let mut cursor = cursor.cursor_advancer();

        let res = if let Some(res) = cursor.peek() {
            res.into_owned()
        } else {
            break;
        };
        if res.file_id != room_id {
            continue;
        };
        if res.fourcc() != reader_writer::FourCC::from_bytes(b"MREA") {
            continue;
        };

        let mut area = editor.get_area(&mut cursor);
        let scly = area.mrea().scly_section_mut().clone();
        for scly_layer in scly.layers.iter() {
            for obj in scly_layer.objects.iter() {
                let obj = obj.into_owned();
                //obj.property_data.guess_kind();
                object_map.insert(obj.instance_id, obj);
            }
        }
    }

    fn read_name(obj: &structs::SclyObject) -> String
    {
        match obj.property_data {
            structs::SclyProperty::Unknown { ref data, .. } => {
                let mut reader = data.clone();
                reader.read::<u32>(());
                reader
                    .read::<reader_writer::CStr>(())
                    .to_str()
                    .unwrap()
                    .to_string()
            }
            _ => panic!(),
        }
    }
    fn indent_str(indent: usize) -> String
    {
        let mut s = String::with_capacity(indent);
        for _i in 0..indent {
            s.push(' ');
        }
        s
    }
    fn trace_connections_inner(
        obj: &structs::SclyObject,
        object_map: &HashMap<u32, structs::SclyObject>,
        seen_objects: &mut HashSet<u32>,
        indent: usize,
    )
    {
        println!(
            "{}{:?} - {}:",
            indent_str(indent),
            read_name(obj),
            obj.instance_id
        );
        for con in obj.connections.iter() {
            if !object_map.contains_key(&con.target_object_id) {
                println!(
                    "{}??? - {}...",
                    indent_str(indent + 4),
                    con.target_object_id
                );
            } else if seen_objects.contains(&con.target_object_id) {
                let ref obj = object_map[&con.target_object_id];
                println!(
                    "{}{:?} - {}...",
                    indent_str(indent + 4),
                    read_name(obj),
                    con.target_object_id
                );
            } else {
                seen_objects.insert(con.target_object_id);
                trace_connections_inner(
                    &object_map[&con.target_object_id],
                    object_map,
                    seen_objects,
                    indent + 4,
                );
            }
        }
    }
    let ref obj = object_map[&instance_id];
    let mut seen_objects = HashSet::new();
    seen_objects.insert(instance_id);
    trace_connections_inner(obj, &object_map, &mut seen_objects, 0);
}

fn print_all_instance_ids(gc_disc: &mut structs::GcDisc, pak_name: &str)
{
    let file_entry = gc_disc.find_file(pak_name);
    let pak = match *file_entry.file().unwrap() {
        structs::FstEntryFile::Pak(ref pak) => pak.clone(),
        structs::FstEntryFile::Unknown(ref reader) => reader.clone().read(()),
        _ => panic!(),
    };

    let mut resources = pak.resources;
    let mlvl = resources
        .iter()
        .find(|i| i.fourcc() == reader_writer::FourCC::from_bytes(b"MLVL"))
        .unwrap()
        .kind
        .as_mlvl()
        .unwrap()
        .into_owned();

    let mut editor = mlvl_wrapper::MlvlEditor::new(mlvl);

    //for res in resources.iter() {
    let mut cursor = resources.cursor();
    loop {
        let mut cursor = cursor.cursor_advancer();

        let res = if let Some(res) = cursor.peek() {
            res.into_owned()
        } else {
            break;
        };
        if res.fourcc() != reader_writer::FourCC::from_bytes(b"MREA") {
            continue;
        };

        let mut area = editor.get_area(&mut cursor);
        let scly = area.mrea().scly_section_mut().clone();

        assert_eq!(scly.layers.len(), area.layer_names.len());
        for (scly_layer, _name) in scly.layers.iter().zip(area.layer_names.iter()) {
            for obj in scly_layer.objects.iter() {
                println!("0x{:08x}", obj.instance_id);
            }
        }
    }
}

fn dump_instance(gc_disc: &mut structs::GcDisc, pak_name: &str, instance_id: u32)
{
    let file_entry = gc_disc.find_file(pak_name);
    let pak = match *file_entry.file().unwrap() {
        structs::FstEntryFile::Pak(ref pak) => pak.clone(),
        structs::FstEntryFile::Unknown(ref reader) => reader.clone().read(()),
        _ => panic!(),
    };

    let mut resources = pak.resources;
    let mlvl = resources
        .iter()
        .find(|i| i.fourcc() == reader_writer::FourCC::from_bytes(b"MLVL"))
        .unwrap()
        .kind
        .as_mlvl()
        .unwrap()
        .into_owned();

    println!("{:#?}", mlvl.memory_relays);

    let mut editor = mlvl_wrapper::MlvlEditor::new(mlvl);

    //for res in resources.iter() {
    let mut cursor = resources.cursor();
    loop {
        let mut cursor = cursor.cursor_advancer();

        let res = if let Some(res) = cursor.peek() {
            res.into_owned()
        } else {
            break;
        };
        if res.fourcc() != reader_writer::FourCC::from_bytes(b"MREA") {
            continue;
        };

        let mut area = editor.get_area(&mut cursor);

        //println!("{}.MREA - {:b}", res.file_id, area.layer_flags.flags);

        let scly = area.mrea().scly_section_mut().clone();

        assert_eq!(scly.layers.len(), area.layer_names.len());
        for (scly_layer, _name) in scly.layers.iter().zip(area.layer_names.iter()) {
            //println!("  {:?}: {}", name, scly_layer.objects.len());
            for obj in scly_layer.objects.iter() {
                if obj.instance_id != instance_id {
                    continue;
                }

                let reader = match obj.property_data {
                    structs::SclyProperty::Unknown {
                        ref data,
                        object_type,
                        ..
                    } => {
                        println!("{}", object_type);
                        data.clone()
                    }
                    _ => panic!(),
                };
                use std::io::Write;
                let mut file = File::create("/tmp/pickup").unwrap();
                file.write_all(&reader).unwrap();
            }
        }
    }
}

fn resource_ids(gc_disc: &mut structs::GcDisc, pak_name: &str) -> Vec<u32>
{
    let pak = match *gc_disc.find_file(pak_name).file().unwrap() {
        structs::FstEntryFile::Pak(ref pak) => pak.clone(),
        structs::FstEntryFile::Unknown(ref reader) => reader.clone().read(()),
        _ => unreachable!(),
    };
    pak.resources.iter().map(|res| res.file_id).collect()
}

fn dump_pak(gc_disc: &mut structs::GcDisc, pak_name: &str)
{
    let reader = match *gc_disc.find_file(pak_name).file().unwrap() {
        structs::FstEntryFile::Unknown(ref reader) => reader.clone(),
        _ => unreachable!(),
    };
    let mut file = File::create(pak_name).unwrap();
    file.write_all(&reader).unwrap();
}
/*
    1310957,// Power Bomb Expansion
    600301,// Power Bomb Expansion
    3604505,// Power Bomb Expansion
    918079,// Power Bomb Expansion
*/

static CUT_SCENE_PICKUPS: &'static [u32] = &[
    589860,    // Morph Ball
    1377077,   // Wavebuster
    1769497,   // Artifact of Lifegiver
    2359772,   // Missile Launcher
    2435310,   // Varia Suit
    405090173, // Artifact of Wild
    2687109,   // Charge Beam
    3155850,   // Morph Ball Bomb
    3735555,   // Artifact of World
    3997699,   // Ice Beam
    524887,    // Artifact of Sun
    917592,    // Wave Beam
    1048801,   // Boost Ball
    1573322,   // Spider Ball
    1966838,   // Super Missile
    2557135,   // Artifact of Elder
    69730588,  // Thermal Visor
    3473439,   // Gravity Suit
    3539113,   // Artifact of Spirit
    262151,    // Space Jump Boots
    68157908,  // Artifact of Truth
    2752545,   // X-Ray Visor
    2753076,   // Artifact of Chozo
    589827,    // Grapple Beam
    786470,    // Flamethrower
    852800,    // Artifact of Warrior
    2556031,   // Artifact of Newborn
    272508,    // Artifact of Nature
    720951,    // Artifact of Strength
    786472,    // Ice Spreader
    1376287,   // Plasma Bea
];

// fn

fn write_resource_to_file(gc_disc: &mut structs::GcDisc, pak_name: &str, file_id: u32)
{
    let file_entry = gc_disc.find_file(pak_name);
    let pak = match *file_entry.file().unwrap() {
        structs::FstEntryFile::Pak(ref pak) => pak.clone(),
        structs::FstEntryFile::Unknown(ref reader) => reader.clone().read(()),
        _ => panic!(),
    };

    let mut resources = pak.resources.clone();
    let mut cursor = resources.cursor();
    loop {
        let mut cursor = cursor.cursor_advancer();

        let res = if let Some(res) = cursor.peek() {
            res.clone()
        } else {
            break;
        };
        if res.file_id != file_id {
            continue;
        }
        let reader = match res.kind {
            structs::ResourceKind::Unknown(ref reader, _) => reader,
            _ => panic!(),
        };
        let mut file = File::create("/tmp/res").unwrap();
        use std::io::Write;
        file.write_all(&reader).unwrap();
    }
}

fn print_thp_file(gc_disc: &mut structs::GcDisc, thp_name: &str)
{
    let file_entry = gc_disc.find_file(thp_name);
    let mut thp: structs::Thp = match *file_entry.file().unwrap() {
        // structs::FstEntryFile::Pak(ref pak) => pak.clone(),
        structs::FstEntryFile::Unknown(ref reader) => reader.clone().read(()),
        _ => panic!(),
    };
    thp.frames.as_mut_vec().truncate(2);
    thp.update_sibling_frame_sizes();
    // println!("{:#?}", thp);
    let mut file = File::create("/tmp/res").unwrap();
    use reader_writer::Writable;
    use std::io::Write;
    thp.write(&mut file).unwrap();
    // file.write_all(&).unwrap();
}

fn summarize_hints(gc_disc: &mut structs::GcDisc, pak_name: &str)
{
    let pak = match *gc_disc.find_file(pak_name).file().unwrap() {
        structs::FstEntryFile::Pak(ref pak) => pak.clone(),
        structs::FstEntryFile::Unknown(ref reader) => reader.clone().read(()),
        _ => unreachable!(),
    };
    for res in pak.resources.iter() {
        if res.fourcc() != reader_writer::FourCC::from_bytes(b"HINT") {
            continue;
        };

        let hint = res.kind.as_hint().unwrap().into_owned();
        println!("{}: {:#?}", res.file_id, hint);
    }
}

fn summarize_strgs(gc_disc: &mut structs::GcDisc, pak_name: &str)
{
    let pak = match *gc_disc.find_file(pak_name).file().unwrap() {
        structs::FstEntryFile::Pak(ref pak) => pak.clone(),
        structs::FstEntryFile::Unknown(ref reader) => reader.clone().read(()),
        _ => unreachable!(),
    };
    println!("Resource Count {}", pak.resources.len());
    for res in pak.resources.iter() {
        let mut k = res.kind.clone();
        k.guess_kind();
        if let Some(strg) = k.as_strg() {
            println!("{:08X} {:#?}", res.file_id, strg);
        }
        if let Some(scan) = k.as_scan() {
            println!("{:08X} {:#?}", res.file_id, scan);
        }
        if let Some(scan) = k.as_savw() {
            println!("{:08X} {:#?}", res.file_id, scan);
        }
    }
}

fn dump_mrea_names(gc_disc: &mut structs::GcDisc, pak_name: &str)
{
    let pak = match *gc_disc.find_file(pak_name).file().unwrap() {
        structs::FstEntryFile::Pak(ref pak) => pak.clone(),
        structs::FstEntryFile::Unknown(ref reader) => reader.clone().read(()),
        _ => unreachable!(),
    };
    let mlvl = pak
        .resources
        .iter()
        .find(|res| res.kind.as_mlvl().is_some())
        .unwrap();
    println!("{}", mlvl.file_id);
    let mlvl = mlvl.kind.as_mlvl().unwrap();
    for area in mlvl.areas.iter() {
        let strg = pak
            .resources
            .iter()
            .find(|res| res.file_id == area.area_name_strg)
            .unwrap();
        let strg = strg.kind.as_strg().unwrap();
        println!(
            "    {:X} {} {:?}",
            area.mrea,
            area.area_name_strg,
            strg.string_tables.iter().next().unwrap()
        );
    }
}

fn compute_default_elevator(
    gc_disc: &mut structs::GcDisc,
    pak_name: &str,
    indices: &mut [Option<u8>],
)
{
    let file_entry = gc_disc.find_file(pak_name);
    let pak = match *file_entry.file().unwrap() {
        structs::FstEntryFile::Pak(ref pak) => pak.clone(),
        structs::FstEntryFile::Unknown(ref reader) => reader.clone().read(()),
        _ => panic!(),
    };

    let mlvl = pak
        .resources
        .iter()
        .find(|res| res.kind.as_mlvl().is_some())
        .map(|res| res.file_id)
        .unwrap();
    for res in pak.resources.iter() {
        let mut mrea = if let Some(mrea) = res.kind.as_mrea() {
            (*mrea).clone()
        } else {
            continue;
        };
        let elv = ELEVATORS
            .iter()
            .enumerate()
            .find(|&(_, elv)| elv.mlvl == mlvl && elv.mrea == res.file_id);
        let (i, elv) = if let Some(e) = elv { e } else { continue };
        let scly = mrea.scly_section_mut();
        let obj = scly
            .layers
            .iter()
            .map(|layer| {
                layer
                    .objects
                    .iter()
                    .find(|obj| obj.instance_id == elv.scly_id)
                    .map(|i| (*i).clone())
            })
            .find(|obj| obj.is_some())
            .and_then(|obj| obj)
            .unwrap();
        let obj = obj.property_data.as_world_transporter().unwrap();

        // println!("{:#X} {:#X} {:?}", mlvl, res.file_id, obj);
        let (target_i, _target_elv) = ELEVATORS
            .iter()
            .enumerate()
            .find(|&(_, elv)| elv.mlvl == obj.mlvl && elv.mrea == obj.mrea)
            .unwrap();
        indices[i] = Some(target_i as u8);
        /* let obj = if let Some(obj) = obj {
            obj
        } else {
            continue
        };*/
    }
}

fn dump_bnr(gc_disc: &mut structs::GcDisc)
{
    let file_entry = gc_disc.find_file("opening.bnr");
    let bnr = match *file_entry.file().unwrap() {
        structs::FstEntryFile::Bnr(ref bnr) => bnr.clone(),
        structs::FstEntryFile::Unknown(ref reader) => reader.clone().read(()),
        _ => panic!(),
    };
    // println!("{:#?}", bnr);
    println!("{:?}", ::std::str::from_utf8(&bnr.game_name));
}

fn main()
{
    // dump_ciso();
    // dump_gcz();

    let file = File::open(args().nth(1).unwrap()).unwrap();
    let mmap = memmap::Mmap::open(&file, memmap::Protection::Read).unwrap();
    let mut reader = Reader::new(unsafe { mmap.as_slice() });
    let mut gc_disc: structs::GcDisc = reader.read(());

    let mut vec = vec![];
    vec.push(gc_disc.header.console_id);
    vec.extend_from_slice(&*gc_disc.header.game_code);
    vec.push(gc_disc.header.country_code);
    vec.extend_from_slice(&*gc_disc.header.maker_code);

    println!("{:?}", std::str::from_utf8(vec.as_slice()));
    println!("{}", gc_disc.header.disc_id);
    println!("{}", gc_disc.header.version);

    // dump_bnr(&mut gc_disc);

    // print_thp_file(&mut gc_disc, "attract9.thp");

    // write_resource_to_file(&mut gc_disc,  "Metroid2.pak", 0x50535432);

    // print_fst(&gc_disc);
    // summarize_resources(&mut gc_disc, "metroid5.pak");
    // summarize_strgs(&mut gc_disc, "Metroid4.pak");// 465453896
    // summarize_resources(&mut gc_disc, "MiscData.pak");
    // summarize_diff_mreas(&mut gc_disc, "Metroid4.pak");
    dump_scly_data(&mut gc_disc, "Metroid4.pak");
    // summarize_hints(&mut gc_disc, "NoARAM.pak");

    // trace_object_connections(&mut gc_disc, "metroid5.pak", 4272124642, 1770673);

    // trace_object_connections(&mut gc_disc, "Metroid4.pak", 597223686, 1209008983);
    // trace_object_connections(&mut gc_disc, "Metroid4.pak", 597223686, 1380007680);
    // trace_object_connections(&mut gc_disc, "Metroid2.pak", 1014518864, 589860);
    // trace_object_connections(&mut gc_disc, "Metroid2.pak", 1014518864, 589860);

    // Power Bomb Expansion?
    // trace_object_connections(&mut gc_disc, "Metroid2.pak", 0x491BFABA, 1310957);

    // Missile Launcher
    // trace_object_connections(&mut gc_disc, "Metroid2.pak", 0xC8309DF6, 2359772);

    // Morph ball
    // trace_object_connections(&mut gc_disc, "Metroid2.pak", 1014518864, 589860);

    // Varia suit
    // trace_object_connections(&mut gc_disc, "Metroid2.pak", 0x9A0A03EB, 2435310);

    // Artifact of Lifegiver
    // trace_object_connections(&mut gc_disc, "Metroid2.pak", 0x11BD63B7, 1769497);

    // Artifact of Sun
    // trace_object_connections(&mut gc_disc, "Metroid3.pak", 0x6655F51E, 524887);

    // Artifact of Sun
    // trace_object_connections(&mut gc_disc, "Metroid3.pak", 0xB3C33249, 2557135);

    // Gravity Suit
    // trace_object_connections(&mut gc_disc, "Metroid3.pak", 0x49175472,  3473439);

    // Artifact of Chozo
    // trace_object_connections(&mut gc_disc, "Metroid4.pak", 0x86EB2E02,  2753076);

    // Artifact of Warrior
    // trace_object_connections(&mut gc_disc, "metroid5.pak", 0x8A97BB54,  852800);

    // Artifact of Newborn
    // trace_object_connections(&mut gc_disc, "metroid5.pak", 0xBBFA4AB3,  2556031);

    // trace_object_connections(&mut gc_disc, "metroid5.pak", 0x89A6CB8D,  720951);

    // Thermal Visor
    // trace_object_connections(&mut gc_disc, "Metroid3.pak", 0xA49B2544,  69730588);

    /*print_all_instance_ids(&mut gc_disc, "Metroid2.pak");
    print_all_instance_ids(&mut gc_disc, "Metroid3.pak");
    print_all_instance_ids(&mut gc_disc, "Metroid4.pak");
    print_all_instance_ids(&mut gc_disc, "metroid5.pak");
    print_all_instance_ids(&mut gc_disc, "Metroid6.pak");
    */
    //dump_instance(&mut gc_disc, "Metroid4.pak", 2753076);

    /*
    let mut res_id_set = HashSet::new();
    res_id_set.extend(resource_ids(&mut gc_disc, "Metroid2.pak"));
    res_id_set.extend(resource_ids(&mut gc_disc, "Metroid3.pak"));
    res_id_set.extend(resource_ids(&mut gc_disc, "Metroid4.pak"));
    res_id_set.extend(resource_ids(&mut gc_disc, "metroid5.pak"));
    res_id_set.extend(resource_ids(&mut gc_disc, "Metroid6.pak"));

    let mut ids: Vec<_> = res_id_set.iter().collect();
    ids.sort();
    for id in ids {
        println!("0x{:08X}", id);
    }
    */

    /*
    const METROID_PAK_NAMES: [&'static str; 5] = [
        "Metroid2.pak",
        "Metroid3.pak",
        "Metroid4.pak",
        "metroid5.pak",
        "Metroid6.pak",
    ];
    for (name, rooms) in METROID_PAK_NAMES.iter().zip(pickup_meta::PICKUP_LOCATIONS.iter()) {
        for room in rooms.iter() {
            for loc in room.pickup_locations.iter() {
                trace_object_connections(&mut gc_disc, name, room.room_id, loc.location.instance_id);
            }
        }
    }
    */

    /*
    let mut elevator_indices = [None; 20];

    const METROID_PAK_NAMES: [&'static str; 6] = [
        "Metroid2.pak",
        "Metroid3.pak",
        "Metroid4.pak",
        "metroid5.pak",
        "Metroid6.pak",
        "Metroid7.pak",
    ];
    for name in METROID_PAK_NAMES.iter() {
        println!("!{}", name);
        // dump_mrea_names(&mut gc_disc, name);
        // dump_scly_data(&mut gc_disc, name);
        summarize_strgs(&mut gc_disc, name);
        // compute_default_elevator(&mut gc_disc, name, &mut elevator_indices);
    }
    println!("{:?}", elevator_indices.iter().map(|i| i.unwrap()).collect::<Vec<u8>>());

    */

    /* for elv in ELEVATORS.iter() {
        println!("{}", ELEVATORS[elv.default_dest as usize].name);
    }*/

    /*
        for (i, elv) in ELEVATORS.iter().enumerate() {
            println!("\
    <tr>
      <td>{}</td>
      <td>{}</td>
      <td>{}</td>
    </tr>", i, elv.name, elv.default_dest);
        }
        */
}

/*
const ELEVATORS: &'static [(&'static str, u32, u32, u32)] = &[
    ("Metroid2.pak", 0x83f6ff6f, 0x3e6b2bb7, 0x007d), //Transport to Tallon Overworld North
    ("Metroid2.pak", 0x83f6ff6f, 0x8316edf5, 0x180027), //Transport to Magmoor Caverns North
    ("Metroid2.pak", 0x83f6ff6f, 0xa5fa69a1, 0x3e002c), //Transport to Tallon Overworld East
    ("Metroid2.pak", 0x83f6ff6f, 0x236e1b0f, 0x3f0028), //Transport to Tallon Overworld South

    ("Metroid3.pak", 0xa8be6291, 0xc00e3781, 0x002d), //Transport to Magmoor Caverns West
    ("Metroid3.pak", 0xa8be6291, 0xdd0b0739, 0x1d005a),// Transport to Magmoor Caverns South

    ("Metroid4.pak", 0xa8be6291, 0x11a02448, 0xe0005),// Transport to Chozo Ruins West
    // XXX Two?
    // ("Metroid4.pak", 0x39f2de28, 0x2398e906, 0x1002d1), // Artifact Temple
    ("Metroid4.pak", 0x39f2de28, 0x2398e906, 0x1002da), // Artifact Temple
    ("Metroid4.pak", 0x39f2de28, 0x8a31665e, 0x160038),// Transport to Chozo Ruins East
    ("Metroid4.pak", 0x39f2de28, 0x15d6ff8b, 0x170032),// Transport to Magmoor Caverns East
    ("Metroid4.pak", 0x39f2de28, 0xca514f0, 0x290024),// Transport to Chozo Ruins South
    ("Metroid4.pak", 0x39f2de28, 0x7d106670, 0x2b0023),// Transport to Phazon Mines East

    ("metroid5.pak", 0xb1ac4d65, 0x430e999c, 0x001c),// Transport to Tallon Overworld South
    ("metroid5.pak", 0xb1ac4d65, 0xe2c2cf38, 0x190011),// Transport to Magmoor Caverns South

    ("Metroid6.pak", 0x3ef8237c, 0x3beaadc9, 0x001f),// Transport to Chozo Ruins North
    ("Metroid6.pak", 0x3ef8237c, 0xdca9a28b, 0xd0022),// Transport to Phendrana Drifts North
    ("Metroid6.pak", 0x3ef8237c, 0x4c3d244c, 0x100020),// Transport to Tallon Overworld West
    ("Metroid6.pak", 0x3ef8237c, 0xef2f1440, 0x1a0024),// Transport to Phazon Mines West
    ("Metroid6.pak", 0x3ef8237c, 0xc1ac9233, 0x1b0028),// Transport to Phendrana Drifts South

    ("Metroid7.pak", 0xc13b09d1, 0x93668996, 0x0098),// Crater Entry Point
    // ("Metroid7.pak", 0xc13b09d1, 0x1a666c55, 0xb0182),// Metroid Prime Lair

];
*/
