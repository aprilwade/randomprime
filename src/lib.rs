#![recursion_limit = "128"]

pub use structs;
pub use reader_writer;
pub use memmap;

use reader_writer::{
    LCow,
    Reader,
};


use enum_map::EnumMap;
use flate2::{Decompress, FlushDecompress};
use num_bigint::BigUint;
use num_integer::Integer;
use num_traits::ToPrimitive;

use std::{
    borrow::Cow,
    collections::hash_map::DefaultHasher,
    ffi::{CStr, CString},
    hash::Hasher,
    iter,
};

pub mod c_interface;
pub mod custom_assets;
pub mod ciso_writer;
pub mod dol_patcher;
pub mod elevators;
pub mod gcz_writer;
pub mod mlvl_wrapper;
pub mod patcher;
pub mod patches;
pub mod pickup_meta;
pub mod starting_items;
pub mod txtr_conversions;

use crate::pickup_meta::PickupType;
use crate::elevators::{Elevator, SpawnRoom};

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

pub fn extract_flaahgra_music_files(iso_path: &str) -> Result<[nod_wrapper::FileWrapper; 2], String>
{
    let res = (|| {
        let dw = nod_wrapper::DiscWrapper::new(iso_path)?;
        Ok([
            dw.open_file(CStr::from_bytes_with_nul(b"rui_flaaghraR.dsp\0").unwrap())?,
            dw.open_file(CStr::from_bytes_with_nul(b"rui_flaaghraL.dsp\0").unwrap())?,
        ])
    })();
    res.map_err(|s: String| format!("Failed to extract Flaahgra music files: {}", s))
}

pub fn parse_layout_chars_to_ints<I>(bytes: &[u8], layout_data_size: usize, checksum_size: usize, is: I)
    -> Result<Vec<u8>, String>
    where I: Iterator<Item = u8> + Clone
{
    const LAYOUT_CHAR_TABLE: [u8; 64] =
        *b"ABCDEFGHIJKLMNOPQRSTUWVXYZabcdefghijklmnopqrstuwvxyz0123456789-_";

    let mut sum: BigUint = 0u8.into();
    for c in bytes.iter().rev() {
        if let Some(idx) = LAYOUT_CHAR_TABLE.iter().position(|i| i == c) {
            sum = sum * BigUint::from(64u8) + BigUint::from(idx);
        } else {
            return Err(format!("Layout contains invalid character '{}'.", c));
        }
    }

    // Reverse the order of the odd bits
    let mut bits = sum.to_str_radix(2).into_bytes();
    for i in 0..(bits.len() / 4) {
        let len = bits.len() - bits.len() % 2;
        bits.swap(i * 2 + 1, len - i * 2 - 1);
    }
    sum = BigUint::parse_bytes(&bits, 2).unwrap();

    // The upper `checksum_size` bits are a checksum, so seperate them from the sum.
    let checksum_bitmask = (1u8 << checksum_size) - 1;
    let checksum = sum.clone() & (BigUint::from(checksum_bitmask) << layout_data_size);
    sum -= checksum.clone();
    let checksum = (checksum >> layout_data_size).to_u8().unwrap();

    let mut computed_checksum = 0;
    {
        let mut sum = sum.clone();
        while sum > 0u8.into() {
            let remainder = (sum.clone() & BigUint::from(checksum_bitmask)).to_u8().unwrap();
            computed_checksum = (computed_checksum + remainder) & checksum_bitmask;
            sum >>= checksum_size;
        }
    }
    if checksum != computed_checksum {
        return Err("Layout checksum failed.".to_string());
    }

    let mut res = vec![];
    for denum in is {
        let (quotient, remainder) = sum.div_rem(&denum.into());
        res.push(remainder.to_u8().unwrap());
        sum = quotient;
    }

    assert!(sum == 0u8.into());

    res.reverse();
    Ok(res)
}


#[derive(Clone, Debug)]
pub struct Layout
{
    pickups: Vec<PickupType>,
    starting_location: SpawnRoom,
    elevators: EnumMap<Elevator, SpawnRoom>,
    seed: u64,
}

impl std::str::FromStr for Layout {
    type Err = String;
    fn from_str(text: &str) -> Result<Layout, String>
    {
        if !text.is_ascii() {
            return Err("Layout string contains non-ascii characters.".to_string());
        }
        let text = text.as_bytes();

        let (elevator_bytes, pickup_bytes) = if let Some(n) = text.iter().position(|c| *c == b'.') {
            (&text[..n], &text[(n + 1)..])
        } else {
            (b"qzoCAr2fwehJmRjM" as &[u8], text)
        };

        if elevator_bytes.len() != 16 {
            let msg = "The section of the layout string before the '.' should be 16 characters";
            return Err(msg.to_string());
        }

        let (pickup_bytes, has_scan_visor) = if pickup_bytes.starts_with(b"!") {
            (&pickup_bytes[1..], true)
        } else {
            (pickup_bytes, false)
        };
        if pickup_bytes.len() != 87 {
            return Err("Layout string should be exactly 87 characters".to_string());
        }

        // XXX The distribution on this hash probably isn't very good, but we don't use it for anything
        //     particularly important anyway...
        let mut hasher = DefaultHasher::new();
        hasher.write(elevator_bytes);
        hasher.write(pickup_bytes);
        let seed = hasher.finish();

        let pickup_layout = parse_layout_chars_to_ints(
                pickup_bytes,
                if has_scan_visor { 521 } else { 517 },
                if has_scan_visor { 1 } else { 5 },
                iter::repeat(if has_scan_visor { 37u8 } else { 36u8 }).take(100)
            ).map_err(|err| format!("Parsing pickup layout: {}", err))?;
        let pickups = pickup_layout.iter()
            .map(|i| PickupType::from_idx(*i as usize).unwrap())
            .collect();

        let elevator_nums = parse_layout_chars_to_ints(
                elevator_bytes,
                91, 5,
                iter::once(21u8).chain(iter::repeat(20u8).take(20))
            ).map_err(|err| format!("Parsing elevator layout: {}", err))?;

        let starting_location = SpawnRoom::from_u32(*elevator_nums.last().unwrap() as u32)
            .unwrap();
        let mut elevators = EnumMap::<Elevator, SpawnRoom>::new();
        elevators.extend(elevator_nums[..(elevator_nums.len() - 1)].iter()
            .zip(Elevator::iter())
            .map(|(i, elv)| (elv, SpawnRoom::from_u32(*i as u32).unwrap()))
        );

        Ok(Layout {
            pickups,
            starting_location,
            elevators,
            seed,
        })
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
