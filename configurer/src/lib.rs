pub extern crate structs;

pub use structs::reader_writer;

pub fn find_file<'r, 'a: 'r>(gc_disc: &'r structs::GcDisc<'a>, name: &str)
    -> &'r structs::FstEntry<'a>
{
    let fst = &gc_disc.file_system_table;
    fst.fst_entries.iter()
        .find(|e| e.name.to_bytes() == name.as_bytes())
        .unwrap()
}

pub fn find_file_mut<'r, 'a: 'r>(gc_disc: &'r mut structs::GcDisc<'a>, name: &str)
    -> &'r mut structs::FstEntry<'a>
{
    let fst = &mut gc_disc.file_system_table;
    fst.fst_entries.iter_mut()
        .find(|e| e.name.to_bytes() == name.as_bytes())
        .unwrap()
}
