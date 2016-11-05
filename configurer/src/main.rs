extern crate structs;
extern crate memmap;

pub use structs::reader_writer;

use reader_writer::{Reader, Readable};

use std::env::args;
use std::fs::{File, OpenOptions};

fn print_fst(gc_disc: &structs::GcDisc)
{
    let fst = &gc_disc.file_system_table;
    let mut entries : Vec<_> = fst.fst_entries.iter()
            .filter(|e| !e.is_folder())
            .collect();
    entries.sort_by(|l, r| l.offset.cmp(&r.offset).reverse());
    for entry in entries.iter() {
        println!("{:?} : {} {}", entry.name, entry.offset, entry.length);
    }
}

fn replace_file_with_external(gc_disc: &mut structs::GcDisc, name: &str, path: &str)
{
    let file = File::open(path).unwrap();
    let len = file.metadata().unwrap().len() as usize;
    let entry = gc_disc.file_system_table.fst_entries.iter_mut()
        .find(|e| e.name.to_bytes() == name.as_bytes())
        .unwrap();
    *entry.file_mut().unwrap() = structs::FstEntryFile::ExternalFile(
        structs::ReadWrapper::new(file),
        len,
    );
}
fn replace_flaahgra_music(gc_disc: &mut structs::GcDisc)
{
    replace_file_with_external(
        gc_disc,
        "rui_flaaghraL.dsp",
        "../rui_flaaghraL.dsp",
    );
    replace_file_with_external(
        gc_disc,
        "rui_flaaghraR.dsp",
        "../rui_flaaghraR.dsp",
    );
}

fn expand_metroid_paks(gc_disc: &mut structs::GcDisc)
{
    let fst = &mut gc_disc.file_system_table;
    for e in fst.fst_entries.iter_mut() {
        if e.name.to_bytes().starts_with(b"metroid") ||
           e.name.to_bytes().starts_with(b"Metroid") {
            let file = e.file_mut().unwrap();
            let reader = match *file {
                structs::FstEntryFile::Unknown(ref reader) => reader.clone(),
                _ => unreachable!(),
            };
            *file = structs::FstEntryFile::Pak(reader.clone().read((reader.len(), ())));
            let pak = match *file {
                structs::FstEntryFile::Pak(ref mut pak) => &mut **pak,
                _ => panic!(),
            };

            // TODO: Partially randomize this?
            println!("{}", pak.resources.len());
            let mut resources_cursor = pak.resources.cursor();
            loop {
                if resources_cursor.value().is_none() {
                    break
                };
                resources_cursor.next()
            }
        }
    }
}

fn find_file<'r, 'a: 'r>(gc_disc: &'r mut structs::GcDisc<'a>, name: &str)
    -> &'r mut structs::FstEntry<'a>
{
    let fst = &mut gc_disc.file_system_table;
    fst.fst_entries.iter_mut()
        .find(|e| e.name.to_bytes() == name.as_bytes())
        .unwrap()
}

fn write_gc_disc(gc_disc: &mut structs::GcDisc)
{
    //replace_flaahgra_music(&mut gc_disc);
    expand_metroid_paks(gc_disc);

    let out_iso = OpenOptions::new()
        .write(true)
        .create(true)
        .open("../output.iso")
        .unwrap();
    out_iso.set_len(structs::GC_DISC_LENGTH as u64).unwrap();

    gc_disc.write(&mut &out_iso);
}

fn main() {
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
    println!("{}", gc_disc.header.size());
    //println!("{:?}", structs::FstEntry::fixed_size());

    write_gc_disc(&mut gc_disc);

    //print_fst(&gc_disc);
}
