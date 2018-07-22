extern crate flate2;
extern crate memmap;
extern crate rand;
extern crate sha2;
extern crate winapi;

pub extern crate structs;

pub use structs::reader_writer;
use reader_writer::{LCow, Reader};
use reader_writer::num::{BigUint, Integer, ToPrimitive};

use flate2::{Decompress, Flush};
use sha2::{Digest, Sha512};

use std::borrow::Cow;
use std::iter;

pub mod elevators;
pub mod mlvl_wrapper;
pub mod pickup_meta;
pub mod patcher;

pub trait GcDiscLookupExtensions<'a>
{
    fn find_file(&self, name: &str) -> &structs::FstEntry<'a>;
    fn find_file_mut(&mut self, name: &str) -> &mut structs::FstEntry<'a>;
    fn find_resource<'r, F>(&'r self, pak_name: &str, f: F)
        -> Option<LCow<'r, structs::Resource<'a>>>
        where F: FnMut(&structs::Resource<'a>) -> bool;
    fn find_resource_mut<'r, F>(&'r mut self, pak_name: &str, f: F)
        -> Option<&'r mut structs::Resource<'a>>
        where F: FnMut(&structs::Resource<'a>) -> bool;
}

impl<'a> GcDiscLookupExtensions<'a> for structs::GcDisc<'a>
{
    fn find_file(&self, name: &str) -> &structs::FstEntry<'a>
    {
        let fst = &self.file_system_table;
        fst.fst_entries.iter()
            .find(|e| e.name.to_bytes() == name.as_bytes())
            .unwrap()
    }

    fn find_file_mut(&mut self, name: &str) -> &mut structs::FstEntry<'a>
    {
        let fst = &mut self.file_system_table;
        fst.fst_entries.iter_mut()
            .find(|e| e.name.to_bytes() == name.as_bytes())
            .unwrap()
    }

    fn find_resource<'r, F>(&'r self, pak_name: &str, mut f: F)
        -> Option<LCow<'r, structs::Resource<'a>>>
        where F: FnMut(&structs::Resource<'a>) -> bool
    {
        let file_entry = self.find_file(pak_name);
        match *file_entry.file()? {
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
        let file_entry = self.find_file_mut(pak_name);
        file_entry.guess_kind();
        let pak = match *file_entry.file_mut()? {
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
    sum = sum - checksum.clone();
    let checksum = (checksum >> layout_data_size).to_u8().unwrap();

    let mut computed_checksum = 0;
    {
        let mut sum = sum.clone();
        while sum > 0u8.into() {
            let remainder = (sum.clone() & BigUint::from(checksum_bitmask)).to_u8().unwrap();
            computed_checksum = (computed_checksum + remainder) & checksum_bitmask;
            sum = sum >> checksum_size;
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


pub fn parse_layout(text: &str) -> Result<(Vec<u8>, Vec<u8>, [u32; 16]), String>
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

    if pickup_bytes.len() != 87 {
        return Err("Layout string should be exactly 87 characters".to_string());
    }

    let mut seed_hasher = Sha512::default();
    seed_hasher.input(elevator_bytes);
    seed_hasher.input(pickup_bytes);
    let seed_data = seed_hasher.result();
    let mut seed_reader = Reader::new(&seed_data);
    let seed = [
        seed_reader.read(()), seed_reader.read(()), seed_reader.read(()), seed_reader.read(()),
        seed_reader.read(()), seed_reader.read(()), seed_reader.read(()), seed_reader.read(()),
        seed_reader.read(()), seed_reader.read(()), seed_reader.read(()), seed_reader.read(()),
        seed_reader.read(()), seed_reader.read(()), seed_reader.read(()), seed_reader.read(()),
    ];

    let pickup_layout = parse_layout_chars_to_ints(
            pickup_bytes,
            517, 5,
            iter::repeat(36u8).take(100)
        ).map_err(|err| format!("Parsing pickup layout: {}", err))?;

    let elevator_layout = parse_layout_chars_to_ints(
            elevator_bytes,
            91, 5,
            iter::once(21u8).chain(iter::repeat(20u8).take(20))
        ).map_err(|err| format!("Parsing elevator layout: {}", err))?;

    Ok((pickup_layout, elevator_layout, seed))
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
            Decompress::new(false).decompress(&reader, &mut output, Flush::Finish).unwrap();

            Cow::Owned(output)
        } else {
            Cow::Borrowed(&self.data)
        }
    }
}

macro_rules! def_asset_ids {
    (@Build { $prev:expr } $id:ident, $($rest:tt)*) => {
        def_asset_ids!(@Build { $prev } $id = $prev + 1, $($rest)*);
    };
    (@Build { $_prev:expr } $id:ident = $e:expr, $($rest:tt)*) => {
        pub const $id: u32 = $e;
        def_asset_ids!(@Build { $id } $($rest)*);
    };
    (@Build { $prev:expr }) => {
    };
    ($($tokens:tt)*) => {
        def_asset_ids!(@Build { 0 } $($tokens)*);
    };
}

pub mod asset_ids {
    def_asset_ids! {
        PHAZON_SUIT_SCAN = 0xDEAF0000,
        PHAZON_SUIT_STRG,
        PHAZON_SUIT_TXTR1,
        PHAZON_SUIT_TXTR2,
        PHAZON_SUIT_CMDL,
        PHAZON_SUIT_ANCS,
        NOTHING_ACQUIRED_HUDMEMO_STRG,
        NOTHING_SCAN_STRG, // 0xDEAF0007
        NOTHING_SCAN,
        NOTHING_TXTR,
        NOTHING_CMDL,
        NOTHING_ANCS,

        SKIP_HUDMEMO_STRG_START,
        SKIP_HUDMEMO_STRG_END = SKIP_HUDMEMO_STRG_START + 37,

        GRAVITY_SUIT_CMDL = 0x95946E41,
        GRAVITY_SUIT_ANCS = 0x27A97006,
        PHAZON_SUIT_ACQUIRED_HUDMEMO_STRG = 0x11BEB861,
        PHAZON_MINES_SAVW = 0x2D52090E,
        ARTIFACT_TEMPLE_MREA = 0x2398E906,
    }
}
