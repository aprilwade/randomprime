#![recursion_limit = "256"]

pub use structs;
pub use reader_writer;
pub use memmap;

use reader_writer::{
    LCow,
    Reader,
};

use flate2::{Decompress, FlushDecompress};

use std::{
    borrow::Cow,
    ffi::CString,
};

pub mod c_interface;
pub mod custom_assets;
pub mod extern_assets;
pub mod ciso_writer;
pub mod dol_patcher;
pub mod elevators;
pub mod gcz_writer;
pub mod mlvl_wrapper;
pub mod patch_config;
pub mod patcher;
pub mod patches;
pub mod pickup_meta;
pub mod door_meta;
pub mod starting_items;
pub mod txtr_conversions;

pub trait GcDiscLookupExtensions<'a>
{
    fn find_file(&self, name: &str) -> Option<&structs::FstEntry<'a>>;
    fn find_file_mut(&mut self, name: &str) -> Option<&mut structs::FstEntry<'a>>;
    fn find_resource<'r, F>(&'r self, pak_name: &str, f: F)
        -> Option<LCow<'r, structs::Resource<'a>>>
        where F: FnMut(&structs::Resource<'a>) -> bool;
    fn find_resource_mut<'r, F>(&'r mut self, pak_name: &str, f: F)
        -> Option<&'r mut structs::Resource<'a>>
        where F: FnMut(&structs::Resource<'a>) -> bool;

    fn add_file(&mut self, path: &str, file: structs::FstEntryFile<'a>) -> Result<(), String>;
}

impl<'a> GcDiscLookupExtensions<'a> for structs::GcDisc<'a>
{
    fn find_file(&self, name: &str) -> Option<&structs::FstEntry<'a>>
    {
        let mut entry = &self.file_system_root;
        for seg in name.split('/') {
            if seg.len() == 0 {
                continue
            }
            match entry {
                structs::FstEntry::Dir(_, entries) => {
                    entry = entries.iter()
                        .find(|e| e.name().to_bytes() == seg.as_bytes())?;
                },
                structs::FstEntry::File(_, _, _) => return None,
            }
        }
        Some(entry)
    }

    fn find_file_mut(&mut self, name: &str) -> Option<&mut structs::FstEntry<'a>>
    {
        let mut entry = &mut self.file_system_root;
        for seg in name.split('/') {
            if seg.len() == 0 {
                continue
            }
            match entry {
                structs::FstEntry::Dir(_, entries) => {
                    entry = entries.iter_mut()
                        .find(|e| e.name().to_bytes() == seg.as_bytes())?;
                },
                structs::FstEntry::File(_, _, _) => return None,
            }
        }
        Some(entry)
    }

    fn find_resource<'r, F>(&'r self, pak_name: &str, mut f: F)
        -> Option<LCow<'r, structs::Resource<'a>>>
        where F: FnMut(&structs::Resource<'a>) -> bool
    {
        let file_entry = self.find_file(pak_name)?;
        match file_entry.file()? {
            structs::FstEntryFile::Pak(ref pak) => pak.resources.iter().find(|res| f(&res)),
            structs::FstEntryFile::Unknown(ref reader) => {
                let pak: structs::Pak = reader.clone().read(());
                pak.resources.iter()
                .find(|res| f(&res))
                .map(|res| LCow::Owned(res.into_owned()))
            },
            _ => panic!(),
        }
    }

    fn find_resource_mut<'r, F>(&'r mut self, pak_name: &str, mut f: F)
        -> Option<&'r mut structs::Resource<'a>>
        where F: FnMut(&structs::Resource<'a>) -> bool
    {
        let file_entry = self.find_file_mut(pak_name)?;
        file_entry.guess_kind();
        let pak = match file_entry.file_mut()? {
            structs::FstEntryFile::Pak(ref mut pak) => pak,
            _ => panic!(),
        };
        let mut cursor = pak.resources.cursor();
        loop {
            if cursor.peek().map(|res| f(&res)).unwrap_or(true) {
                break
            }
            cursor.next();
        }
        cursor.into_value()
    }

    fn add_file(&mut self, path: &str, file: structs::FstEntryFile<'a>) -> Result<(), String>
    {
        let mut split = path.rsplitn(2, '/');
        let file_name = split.next()
            .ok_or_else(|| "".to_owned())?;

        let new_entry = structs::FstEntry::File(
            Cow::Owned(CString::new(file_name).unwrap()),
            file,
            None,
        );
        let path = if let Some(path) = split.next() {
            path
        } else {
            self.file_system_root.dir_entries_mut().unwrap().push(new_entry);
            return Ok(())
        };

        let mut entry = &mut self.file_system_root;
        for seg in path.split('/') {
            if seg.len() == 0 {
                continue
            }
            let dir_entries = entry.dir_entries_mut()
                .ok_or_else(|| "".to_owned())?;

            let maybe_pos = dir_entries
                .iter()
                .position(|e| e.name().to_bytes() == seg.as_bytes());
            if let Some(pos) = maybe_pos {
                entry = &mut dir_entries[pos];
            } else {
                dir_entries.push(structs::FstEntry::Dir(
                    Cow::Owned(CString::new(seg).unwrap()),
                    vec![],
                ));
                entry = dir_entries.last_mut().unwrap()
            }
        }

        entry.dir_entries_mut()
            .ok_or_else(|| "".to_owned())?
            .push(new_entry);
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct ResourceData<'a>
{
    pub is_compressed: bool,
    pub data: Reader<'a>,
}

impl<'a> ResourceData<'a>
{
    pub fn new_external(res: &'a structs::Resource<'_>) -> ResourceData<'a>
    {
        let reader = match &res.kind {
            structs::ResourceKind::External(bytes, _) => Reader::new(&bytes[..]),
            _ => panic!("Only uninitialized (aka Unknown) resources may be added."),
        };
        ResourceData {
            is_compressed: res.compressed,
            data: reader,
        }
    }

    pub fn new(res: &structs::Resource<'a>) -> ResourceData<'a>
    {
        let reader = match res.kind {
            structs::ResourceKind::Unknown(ref reader, _) => reader.clone(),
            _ => panic!("Only uninitialized (aka Unknown) resources may be added."),
        };
        ResourceData {
            is_compressed: res.compressed,
            data: reader,
        }
    }
    pub fn decompress(&self) -> Cow<'a, [u8]>
    {
        if self.is_compressed {
            let mut reader = self.data.clone();
            let size: u32 = reader.read(());
            let _header: u16 = reader.read(());
            // TODO: We could use Vec::set_len to avoid initializing the whole array.
            let mut output = vec![0; size as usize];
            Decompress::new(false).decompress(&reader, &mut output, FlushDecompress::Finish).unwrap();

            Cow::Owned(output)
        } else {
            Cow::Borrowed(&self.data)
        }
    }
}
