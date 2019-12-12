use auto_struct_macros::auto_struct;

use reader_writer::{CStr, Reader, Readable, RoArray, WithRead, Writable};
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;

use std::io::{self, Write};
use std::iter;

use crate::{
    pak::Pak,
    thp::Thp,
    bnr::Bnr,
};

// Based on http://hitmen.c02.at/files/yagcd/yagcd/chap13.html

pub const GC_DISC_LENGTH: usize = 1_459_978_240;

pub struct GcDisc<'r>
{
    pub header: GcDiscHeader,
    header_info: GenericArray<u8, U8192>,
    apploader: GcDiscApploader<'r>,
    pub file_system_root: FstEntry<'r>,
}

impl<'r> Readable<'r> for GcDisc<'r>
{
    type Args = ();
    fn read_from(reader: &mut Reader<'r>, (): ()) -> GcDisc<'r>
    {
        let start = reader.clone();
        let header: GcDiscHeader = reader.read(());
        let header_info = reader.read(());
        let apploader = reader.read(());

        let fst_start = start.offset(header.fst_offset as usize);
        let root_fst_entry: RawFstEntry = fst_start.clone().read(());

        let fst_len = root_fst_entry.length as usize;
        let string_table_start = fst_start.offset(fst_len * RawFstEntry::fixed_size().unwrap());

        let fst = { fst_start }.read((0, start, string_table_start));

        let gc_disc = GcDisc {
            header: header,
            header_info: header_info,
            apploader: apploader,
            file_system_root: fst,
        };
        gc_disc
    }

    fn fixed_size() -> Option<usize>
    {
        Some(0)
    }
}

pub trait ProgressNotifier
{
    fn notify_total_bytes(&mut self, total_size: usize);
    fn notify_writing_file(&mut self, file_name: &CStr, file_bytes: usize);
    fn notify_writing_header(&mut self);
    fn notify_flushing_to_disk(&mut self);
}

pub trait WriteExt
{
    fn skip_bytes(&mut self, bytes: u64) -> io::Result<()>;
}

impl<W> WriteExt for W
    where W: Write + io::Seek
{
    fn skip_bytes(&mut self, bytes: u64) -> io::Result<()>
    {
        self.seek(io::SeekFrom::Current(bytes as i64)).map(|_| ())
    }
}

impl<'r> GcDisc<'r>
{
    pub fn write<W, N>(&mut self, writer: &mut W, notifier: &mut N)
        -> io::Result<()>
        where W: Write + WriteExt,
              N: ProgressNotifier,
    {
        let raw_fst = self.file_system_root.generate_raw_fst_data();
        let header_size = self.header.size() + self.header_info.size() + self.apploader.size();

        let files_offset = raw_fst.iter()
            .filter(|entry| !entry.raw_entry.is_folder())
            .map(|entry| entry.raw_entry.offset)
            .min()
            .unwrap();

        let file_system_size = raw_fst.iter()
            .filter(|entry| !entry.raw_entry.is_folder())
            .map(|entry| entry.raw_entry.length)
            .sum::<u32>() as usize;

        let total_size = (RawFstEntry::fixed_size().unwrap() * raw_fst.len())
            + header_size + file_system_size;
        notifier.notify_total_bytes(total_size);

        let main_dol_offset = raw_fst.iter()
            .find(|e| e.name.to_bytes() == "default.dol".as_bytes())
            .map(|e| e.raw_entry.offset)
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other,
                                          "Couldn't find default.dol".to_owned()))?;

        self.header.main_dol_offset = main_dol_offset;
        self.header.fst_length = self.file_system_root.size() as u32;
        self.header.fst_max_length = self.header.fst_length;

        notifier.notify_writing_header();
        self.header.write_to(writer)?;
        self.header_info.write_to(writer)?;
        self.apploader.write_to(writer)?;

        writer.skip_bytes(self.header.fst_offset as u64 - header_size as u64)?;
        for e in raw_fst.iter() {
            e.raw_entry.write_to(writer)?;
        }
        for e in raw_fst.iter() {
            e.name.write_to(writer)?;
        }

        let fst_end = (self.header.fst_offset + self.header.fst_length) as u64;
        writer.skip_bytes(files_offset as u64 - fst_end)?;
        FstEntry::write_files(writer, notifier, &raw_fst)
    }
}

#[auto_struct(Readable, FixedSize, Writable)]
#[derive(Debug)]
pub struct GcDiscHeader
{
    pub console_id: u8,
    pub game_code: GenericArray<u8, U2>,
    pub country_code: u8,

    pub maker_code: GenericArray<u8, U2>,
    pub disc_id: u8,
    pub version: u8,

    pub audio_streaming: u8,
    pub stream_buffer_size: u8,

    pub unused0: GenericArray<u8, U18>, //[0x12]

    #[auto_struct(expect = 0xc2339f3d)]
    magic: u32, // 0xc2339f3d
    pub game_name: GenericArray<u8, U992>, //[0x3e0]

    pub debug_mon_offset: u32,
    pub debug_mon_load_addr: u32,

    pub unused1: GenericArray<u8, U24>,// [0x18]

    pub main_dol_offset: u32,

    pub fst_offset: u32,
    pub fst_length: u32,
    pub fst_max_length: u32,

    pub user_position: u32,
    pub user_length: u32,

    pub unused2: u32,
    pub unused3: u32,
}

impl GcDiscHeader
{
    pub fn game_identifier(&self) -> [u8; 6]
    {
        [self.console_id, self.game_code[0], self.game_code[1], self.country_code,
         self.maker_code[0], self.maker_code[1]]
    }
}


#[auto_struct(Readable, Writable)]
pub struct GcDiscApploader<'r>
{
    pub date: GenericArray<u8, U16>,
    pub entrypoint: u32,
    pub size: u32,
    pub trailer_size: u32,
    // TODO: Is this size right?
    #[auto_struct(init = ((size + trailer_size) as usize, ()))]
    pub code: RoArray<'r, u8>,
}


#[derive(Clone, Debug)]
pub enum FstEntry<'r>
{
    Dir(CStr<'r>, Vec<FstEntry<'r>>),
    File(CStr<'r>, FstEntryFile<'r>, Option<u32>),
}

impl<'r> Readable<'r> for FstEntry<'r>
{
    type Args = (u32, Reader<'r>, Reader<'r>);
    fn read_from(reader: &mut Reader<'r>, (self_offset, disc_start, string_table): Self::Args)
        -> Self
    {
        let reader_start = reader.clone();
        let raw: RawFstEntry = reader.read(());
        let name = string_table.offset(raw.name_offset as usize).read::<CStr<'r>>(());
        if raw.flags == 1 {
            let mut entries = vec![];
            loop {
                let bytes_read = reader_start.len() - reader.len();
                let index = (bytes_read / RawFstEntry::fixed_size().unwrap()) as u32;
                if index >= (raw.length - self_offset) {
                    break
                }
                entries.push(reader.read((index, disc_start.clone(), string_table.clone())));
            }
            FstEntry::Dir(name, entries)
        } else {
            let file = FstEntryFile::Unknown(
                disc_start.offset(raw.offset as usize).truncated(raw.length as usize)
            );
            FstEntry::File(name, file, Some(raw.offset))
        }
    }

    fn size(&self) -> usize
    {
        self.name().to_bytes_with_nul().len() + match self {
            FstEntry::Dir(_, entries) => RawFstEntry::fixed_size().unwrap() + entries.size(),
            FstEntry::File(_, _, _) => RawFstEntry::fixed_size().unwrap(),
        }
    }
}

impl<'r> FstEntry<'r>
{
    fn generate_raw_fst_data<'a>(&'a self) -> Vec<WrappedFstEntry<'a, 'r>>
    {
        struct S<'a, 'r>
        {
            entries: Vec<WrappedFstEntry<'a, 'r>>,
            parent_index: u32,
            string_table_len: u16,
        }

        fn inner<'a, 'r>(entries: &'a [FstEntry<'r>], state: &mut S<'a, 'r>)
        {
            for entry in entries {
                match entry {
                    FstEntry::Dir(name, entries) => {
                        let dir_entry_idx = state.entries.len();
                        state.entries.push(WrappedFstEntry {
                            raw_entry: RawFstEntry {
                                flags: 1,
                                unused: 0,
                                name_offset: state.string_table_len,
                                offset: state.parent_index,
                                length: 0,
                            },
                            file: None,
                            name: name,
                        });

                        state.string_table_len += name.to_bytes_with_nul().len() as u16;

                        let prev_parent_index = state.parent_index;
                        state.parent_index = dir_entry_idx as u32;
                        inner(entries, state);
                        state.parent_index = prev_parent_index;
                        state.entries[dir_entry_idx].raw_entry.length = state.entries.len() as u32;
                    },
                    FstEntry::File(name, file, original_offset) => {
                        state.entries.push(WrappedFstEntry {
                            raw_entry: RawFstEntry {
                                flags: 0,
                                unused: 0,
                                name_offset: state.string_table_len,
                                offset: original_offset.unwrap_or(0),
                                length: file.size() as u32,
                            },
                            file: Some(file),
                            name: name,
                        });
                        state.string_table_len += name.to_bytes_with_nul().len() as u16;
                    },
                }
            }
        }

        let (root_name, root_vec) = match &self {
            FstEntry::Dir(name, v) => (name, v),
            _ => unreachable!(),
        };

        let mut state = S {
            entries: vec![WrappedFstEntry {
                raw_entry: RawFstEntry {
                    flags: 1,
                    unused: 0,
                    name_offset: 0,
                    offset: 0,
                    length: 0,
                },
                file: None,
                name: root_name,
            }],
            parent_index: 0,
            string_table_len: root_name.to_bytes_with_nul().len() as u16,
        };

        inner(&root_vec, &mut state);
        state.entries[0].raw_entry.length = state.entries.len() as u32;

        // Recompute the on-disc sort order/locations
        let mut entries: Vec<_> = state.entries.iter_mut()
            .filter(|e| !e.raw_entry.is_folder())
            .collect();
        entries.sort_by(|l, r| l.raw_entry.offset.cmp(&r.raw_entry.offset).reverse());
        let mut last_file_offset = GC_DISC_LENGTH as u32;
        for e in entries {
            // We need to round up to a mupliple of 32
            last_file_offset -= (e.raw_entry.length + 31) & (u32::max_value() - 31);
            e.raw_entry.offset = last_file_offset;
        }

        state.entries
    }

    fn write_files<W, N>(writer: &mut W, notifier: &mut N, fst_entries: &[WrappedFstEntry])
        -> io::Result<()>
        where W: Write,
              N: ProgressNotifier,
    {
        let mut entries: Vec<_> = fst_entries.iter()
            .filter(|e| !e.raw_entry.is_folder())
            .collect();
        entries.sort_by(|l, r| l.raw_entry.offset.cmp(&r.raw_entry.offset));

        let mut entries_and_zeroes: Vec<_> = entries[0..entries.len() - 1].iter().zip(entries[1..].iter())
            .map(|(e1, e2)| (*e1, e2.raw_entry.offset - (e1.raw_entry.offset + e1.raw_entry.length)))
            .collect();
        entries_and_zeroes.push((entries[entries.len() - 1], 0));

        let zero_bytes = [0u8; 32];
        for (e, zeroes) in entries_and_zeroes {
            if let Some(f) = e.file {
                notifier.notify_writing_file(&e.name, e.raw_entry.length as usize);
                f.write_to(writer)?;
                writer.write_all(&zero_bytes[0..zeroes as usize])?;
            }
        }
        Ok(())
    }
}

#[auto_struct(Readable, FixedSize, Writable)]
#[derive(Debug)]
struct RawFstEntry
{
    flags: u8,
    unused: u8,
    name_offset: u16,

    offset: u32,
    length: u32,
}

impl RawFstEntry
{
    fn is_folder(&self) -> bool
    {
        self.flags == 1
    }
}


struct WrappedFstEntry<'a, 'r>
{
    raw_entry: RawFstEntry,
    file: Option<&'a FstEntryFile<'r>>,
    name: &'a CStr<'r>,
}


#[derive(Debug, Clone)]
pub enum FstEntryFile<'r>
{
    Pak(Pak<'r>),
    Thp(Thp<'r>),
    Bnr(Bnr<'r>),
    ExternalFile(Box<dyn WithRead + 'r>),
    Unknown(Reader<'r>),
}

impl<'r> FstEntry<'r>
{
    pub fn file(&self) -> Option<&FstEntryFile<'r>>
    {
        match self {
            FstEntry::File(_, file, _) => Some(file),
            _ => None,
        }
    }

    pub fn file_mut(&mut self) -> Option<&mut FstEntryFile<'r>>
    {
        match self {
            FstEntry::File(_, file, _) => Some(file),
            _ => None,
        }
    }

    pub fn dir_entries(&self) -> Option<&[FstEntry<'r>]>
    {
        match self {
            FstEntry::Dir(_, entries) => Some(entries),
            _ => None,
        }
    }

    pub fn dir_entries_mut(&mut self) -> Option<&mut Vec<FstEntry<'r>>>
    {
        match self {
            FstEntry::Dir(_, entries) => Some(entries),
            _ => None,
        }
    }

    pub fn is_folder(&self) -> bool
    {
        match self {
            FstEntry::Dir(_, _) => true,
            FstEntry::File(_, _, _) => false,
        }
    }

    pub fn name(&self) -> &CStr<'r>
    {
        match self {
            FstEntry::Dir(name, _) => name,
            FstEntry::File(name, _, _) => name,
        }
    }

    pub fn guess_kind(&mut self)
    {
        let (name, file) = match self {
            FstEntry::File(name, file, _) => (name, file),
            _ => return,
        };
        let name = name.to_bytes();
        let len = name.len();

        // For simplicity's sake, assume all extentions are len 3
        let mut ext = [name[len - 3], name[len - 2], name[len - 1]];
        ext.make_ascii_lowercase();

        if ext == *b"pak" {
            *file = match file {
                FstEntryFile::Unknown(ref reader)
                    => FstEntryFile::Pak(reader.clone().read(())),
                FstEntryFile::Pak(_) => return,
                _ => panic!("Unexpected fst file type while trying to guess pak."),
            }
        }

        if ext == *b"thp" {
            *file = match file {
                FstEntryFile::Unknown(ref reader)
                    => FstEntryFile::Thp(reader.clone().read(())),
                FstEntryFile::Thp(_) => return,
                _ => panic!("Unexpected fst file type while trying to guess thp."),
            }
        }

        if ext == *b"bnr" {
            *file = match file {
                FstEntryFile::Unknown(ref reader)
                    => FstEntryFile::Bnr(reader.clone().read(())),
                FstEntryFile::Bnr(_) => return,
                _ => panic!("Unexpected fst file type while trying to guess bnr."),
            }
        }
    }

    pub fn dir_files_iter_mut<'a>(&'a mut self) -> DirFilesIterMut<'a, 'r>
    {
        DirFilesIterMut(match self {
            FstEntry::Dir(name, entries) => vec![(name, entries.iter_mut())],
            FstEntry::File(_, _, _) => panic!(),
        })
    }
}

pub struct DirFilesIterMut<'a, 'r>(Vec<(&'a CStr<'r>, core::slice::IterMut<'a, FstEntry<'r>>)>);
impl<'a, 'r> Iterator for DirFilesIterMut<'a, 'r>
{
    type Item = (Vec<u8>, &'a mut FstEntry<'r>);
    fn next(&mut self) -> Option<Self::Item>
    {
        loop {
            let (_, last) = self.0.last_mut()?;
            if let Some(entry) = last.next() {
                match entry {
                    FstEntry::Dir(name, entries) => self.0.push((name, entries.iter_mut())),
                    FstEntry::File(name, _, _) => {
                        let path = self.0[1..].iter()
                            .flat_map(|(n, _)| n.to_bytes().iter().copied().chain(iter::once(b'/')))
                            .chain(name.to_bytes().iter().copied())
                            .collect();
                        return Some((path, entry))
                    },
                }
            } else {
                self.0.pop();
            }
        }
    }
}

impl<'r> FstEntryFile<'r>
{
    fn size(&self) -> usize
    {
        match *self {
            FstEntryFile::Pak(ref pak) => pak.size(),
            FstEntryFile::Thp(ref thp) => thp.size(),
            FstEntryFile::Bnr(ref bnr) => bnr.size(),
            FstEntryFile::ExternalFile(ref i) => i.len(),
            FstEntryFile::Unknown(ref reader) => reader.len(),
        }
    }

    fn write_to<W: io::Write>(&self, writer: &mut W) -> io::Result<u64>
    {
        match *self {
            FstEntryFile::Pak(ref pak) => pak.write_to(writer),
            FstEntryFile::Thp(ref thp) => thp.write_to(writer),
            FstEntryFile::Bnr(ref bnr) => bnr.write_to(writer),
            FstEntryFile::ExternalFile(ref i) => i.with_read(&mut |r| io::copy(r, writer)),
            FstEntryFile::Unknown(ref reader) => {
                writer.write_all(&reader)?;
                Ok(reader.len() as u64)
            },
        }
    }
}
