use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use reader_writer::{LazyArray, Reader, Writable};

use std::{
    borrow::Cow,
    io,
    iter,
    vec,
};

#[derive(Debug, Clone)]
struct BinaryPatcher<'a>
{
    data: &'a [u8],
    patches: Vec<(usize, Cow<'a, [u8]>)>
}

impl<'a> BinaryPatcher<'a>
{
    fn new(data: &'a [u8]) -> BinaryPatcher<'a>
    {
        BinaryPatcher {
            data: data,
            patches: vec![],
        }
    }

    fn patch(&mut self, start: usize, data: Cow<'a, [u8]>) -> Result<(), String>
    {
        for patch in &self.patches {
            if (patch.0 < start && patch.0 + patch.1.len() > start) ||
               (start < patch.0 && start + data.len() > patch.0)
            {
                Err("Overlapping patches".to_owned())?
            }
        }
        self.patches.push((start, data));
        Ok(())
    }


    fn build(mut self) -> PatchedBinary<'a>
    {
        let mut segments = vec![];
        self.patches.sort_by_key(|p| p.0);

        let mut pos = 0;
        for patch in self.patches {
            if pos < patch.0 {
                segments.push(Cow::Borrowed(&self.data[pos..patch.0]));
            }
            pos = patch.0 + patch.1.len();
            segments.push(patch.1);
        }
        if pos < self.data.len() {
            segments.push(Cow::Borrowed(&self.data[pos..]));
        }

        PatchedBinary {
            curr_segment: io::Cursor::new(Cow::Borrowed(&[])),
            segments: segments.into_iter(),
        }
    }

    fn build_ref<'s>(&'s self) -> PatchedBinary<'s>
    {
        let mut segments = vec![];
        let mut patches_refs: Vec<_> = self.patches.iter().collect();
        patches_refs.sort_by_key(|p| p.0);

        let mut pos = 0;
        for patch in patches_refs {
            if pos < patch.0 {
                segments.push(Cow::Borrowed(&self.data[pos..patch.0]));
            }
            pos = patch.0 + patch.1.len();
            segments.push(Cow::Borrowed(&patch.1[..]));
        }
        if pos < self.data.len() {
            segments.push(Cow::Borrowed(&self.data[pos..]));
        }

        PatchedBinary {
            curr_segment: io::Cursor::new(Cow::Borrowed(&[])),
            segments: segments.into_iter(),
        }
    }

    fn len(&self) -> usize
    {
        self.data.len()
    }
}


#[derive(Clone, Debug)]
struct PatchedBinary<'a>
{
    curr_segment: io::Cursor<Cow<'a, [u8]>>,
    segments: vec::IntoIter<Cow<'a, [u8]>>,
}

impl<'a> io::Read for PatchedBinary<'a>
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>
    {
        let mut total_bytes_written = 0;
        loop {
            let offset = self.curr_segment.read(&mut buf[total_bytes_written..])?;
            total_bytes_written += offset;
            // Have we completely filed the buffer yet?
            if total_bytes_written >= buf.len() {
                break;
            }


            if let Some(seg) = self.segments.next() {
                self.curr_segment = io::Cursor::new(seg);
            } else {
                self.curr_segment = io::Cursor::new(Cow::Borrowed(&[]));
                break
            };
        }
        Ok(total_bytes_written)
    }
}

#[derive(Debug, Clone)]
enum DolSegment<'a>
{
    PatchedSegment(u32, BinaryPatcher<'a>),
    NewSegment(u32, Cow<'a, [u8]>),
    EmptySegment,
}

impl<'a> DolSegment<'a>
{
    fn new(seg: &structs::DolSegment<'a>) -> DolSegment<'a>
    {
        let bytes = match &seg.contents {
            LazyArray::Borrowed(array) => &array.data_start()[..],
            _ => unreachable!(),
        };
        if seg.load_addr == 0 {
            DolSegment::EmptySegment
        } else {
            DolSegment::PatchedSegment(seg.load_addr, BinaryPatcher::new(bytes))
        }
    }

    fn is_empty(&self) -> bool
    {
        match self {
            DolSegment::EmptySegment => true,
            _ => false,
        }
    }

    fn len(&self) -> u32
    {
        match self {
            DolSegment::PatchedSegment(_, patcher) => patcher.len() as u32,
            DolSegment::NewSegment(_, bytes) => bytes.len() as u32,
            DolSegment::EmptySegment => 0,
        }
    }

    fn addr(&self) -> u32
    {
        match self {
            DolSegment::PatchedSegment(addr, _) => *addr,
            DolSegment::NewSegment(addr, _) => *addr,
            DolSegment::EmptySegment => 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DolPatcher<'a>
{
    bss_addr: u32,
    bss_size: u32,
    entry_point: u32,
    data_segments: GenericArray<DolSegment<'a>, U11>,
    text_segments: GenericArray<DolSegment<'a>, U7>,
}

impl<'a> DolPatcher<'a>
{
    pub fn new(mut reader: Reader<'a>) -> DolPatcher<'a>
    {
        let dol: structs::Dol = reader.read(());
        DolPatcher {
            bss_addr: dol.bss_addr,
            bss_size: dol.bss_size,
            entry_point: dol.entry_point,
            data_segments: dol.data_segments.iter()
                .map(|seg| DolSegment::new(&seg))
                .collect(),
            text_segments: dol.text_segments.iter()
                .map(|seg| DolSegment::new(&seg))
                .collect(),
        }
    }

    pub fn iter_section_addrs_and_sizes<'s>(&'s self) -> impl Iterator<Item = (u32, u32)> + 's
    {
        iter::once((self.bss_addr, self.bss_size))
            .chain(self.data_segments.iter()
                .filter(|seg| !seg.is_empty())
                .map(|seg| (seg.addr(), seg.len())))
            .chain(self.text_segments.iter()
                .filter(|seg| !seg.is_empty())
                .map(|seg| (seg.addr(), seg.len())))
    }

    fn check_for_overlapping_segment(&self, addr: u32, len: u32) -> Result<(), String>
    {
        let check_overlap = |seg: &DolSegment| {
            (addr <= seg.addr() && addr + len > seg.addr()) ||
                (seg.addr() <= addr && seg.addr() + seg.len() > addr)
        };
        for (i, seg) in self.data_segments.iter().enumerate() {
            if check_overlap(seg) {
                Err(format!("New segment at {:x} overlaps with data segment {} at {:x}",
                            addr, i, seg.addr()))?
            }
        }
        for (i, seg) in self.text_segments.iter().enumerate() {
            if check_overlap(seg) {
                Err(format!("New segment at {:x} overlaps with data segment {} at {:x}",
                            addr, i, seg.addr()))?
            }
        }
        Ok(())
    }

    pub fn add_data_segment(&mut self, addr: u32, bytes: Cow<'a, [u8]>) -> Result<&mut Self, String>
    {
        if bytes.len() & 0x1f != 0 {
            Err(format!("Invalid length for new data ({}) - not 32 byte aligned", bytes.len()))?;
        }
        self.check_for_overlapping_segment(addr, bytes.len() as u32)?;
        let seg = self.data_segments.iter_mut()
            .find(|seg| seg.is_empty())
            .ok_or_else(|| format!("Insufficent room for new data segment"))?;
        *seg = DolSegment::NewSegment(addr, bytes);
        Ok(self)
    }

    pub fn add_text_segment(&mut self, addr: u32, bytes: Cow<'a, [u8]>) -> Result<&mut Self, String>
    {
        if bytes.len() & 0x1f != 0 {
            Err(format!("Invalid length for new text ({}) - not 32 byte aligned", bytes.len()))?;
        }
        self.check_for_overlapping_segment(addr, bytes.len() as u32)?;
        let seg = self.text_segments.iter_mut()
            .find(|seg| seg.is_empty())
            .ok_or_else(|| format!("Insufficent room for new text segment"))?;
        *seg = DolSegment::NewSegment(addr, bytes);
        Ok(self)
    }

    pub fn add_ppcasm_text_segment<A, L>(&mut self, asm: &ppcasm::AsmBlock<A, L>) -> Result<&mut Self, String>
        where A: AsRef<[u32]>
    {
        let mut bytes = asm.encoded_bytes();
        if bytes.len() & 0x1f != 0 {
            let bytes_needed = ((bytes.len() + 31) & !31) - bytes.len();
            bytes.extend([0; 32][..bytes_needed].iter().copied());
        }
        self.add_text_segment(asm.addr(), bytes.into())
    }


    pub fn patch(&mut self, start: u32, data: Cow<'a, [u8]>) -> Result<&mut Self, String>
    {
        let mut matching_seg = None;
        for seg in self.text_segments.iter_mut().chain(&mut self.data_segments) {
            if start > seg.addr() && start < seg.addr() + seg.len()
               && start + data.len() as u32 <= seg.addr() + seg.len() {
                matching_seg = Some(seg);
                break;
            }
        }

        let (addr, patcher) = match matching_seg {
            Some(DolSegment::PatchedSegment(addr, patcher)) => (addr, patcher),
            Some(DolSegment::NewSegment(_, _)) => {
                Err("Patches may not be applied to new segments".to_owned())?
            },
            Some(DolSegment::EmptySegment) => unreachable!(),
            None => Err(format!("Failed to find segment to patch at {:x}", start))?,
        };

        patcher.patch((start - *addr) as usize, data)?;
        Ok(self)
    }

    pub fn ppcasm_patch<A, L>(&mut self, asm: &ppcasm::AsmBlock<A, L>) -> Result<&mut Self, String>
        where A: AsRef<[u32]>
    {
        self.patch(asm.addr(), asm.encoded_bytes().into())
    }
}



impl<'a> reader_writer::WithRead for DolPatcher<'a>
{
    fn len(&self) -> usize
    {
        let contents_len: u32 = self.data_segments.iter().chain(&self.text_segments)
            .map(|seg| seg.len())
            .sum();
        0x100 + contents_len as usize
    }

    fn boxed<'s>(&self) -> Box<dyn reader_writer::WithRead + 's>
        where Self: 's
    {
        Box::new(self.clone())
    }

    fn with_read(&self, f: &mut dyn FnMut(&mut dyn io::Read) -> io::Result<u64>) -> io::Result<u64>
    {
        let mut data_segment_refs = GenericArray::<_, U11>::from_exact_iter(&self.data_segments).unwrap();
        data_segment_refs.sort_by_key(|seg| if seg.is_empty() { 0xFFFFFFFF } else { seg.addr() });
        let mut text_segment_refs = GenericArray::<_, U7>::from_exact_iter(&self.text_segments).unwrap();
        text_segment_refs.sort_by_key(|seg| if seg.is_empty() { 0xFFFFFFFF } else { seg.addr() });

        let segment_refs: GenericArray<_, U18> = text_segment_refs.iter()
            .chain(data_segment_refs.iter())
            .map(|i| *i)
            .collect();

        let mut header = [0; 0x100];
        let mut header = io::Cursor::new(&mut header[..]);

        let mut offset = 0x100;
        for seg in segment_refs.iter() {
            if !seg.is_empty() {
                offset.write_to(&mut header)?;
            } else {
                0u32.write_to(&mut header)?;
            }
            offset += seg.len();
        }
        for seg in segment_refs.iter() {
            seg.addr().write_to(&mut header)?;
        }
        for seg in segment_refs.iter() {
            seg.len().write_to(&mut header)?;
        }
        self.bss_addr.write_to(&mut header)?;
        self.bss_size.write_to(&mut header)?;
        self.entry_point.write_to(&mut header)?;

        header.set_position(0);

        let iter = segment_refs.into_iter()
            .filter_map(|seg| {
                match seg {
                    DolSegment::PatchedSegment(_, patcher) => Some(patcher.build_ref()),
                    DolSegment::NewSegment(_, bytes) => Some(BinaryPatcher::new(&bytes[..]).build()),
                    DolSegment::EmptySegment => None,
                }
            });

        f(&mut io::Read::chain(header, ReadIteratorChain::new(iter)))
    }
}

struct ReadIteratorChain<I>
    where I: Iterator,
          I::Item: io::Read
{
    curr: Option<I::Item>,
    iter: I,
}

impl<I> ReadIteratorChain<I>
    where I: Iterator,
          I::Item: io::Read
{
    fn new(mut iter: I) -> ReadIteratorChain<I>
    {
        let curr = iter.next();
        ReadIteratorChain { curr, iter }
    }
}

impl<I> io::Read for ReadIteratorChain<I>
    where I: Iterator,
          I::Item: io::Read
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>
    {
        let mut total_bytes_written = 0;
        while let Some(curr) = &mut self.curr {
            let offset = curr.read(&mut buf[total_bytes_written..])?;
            total_bytes_written += offset;
            // Have we completely filed the buffer yet?
            if total_bytes_written >= buf.len() {
                break;
            }

            self.curr = self.iter.next();
        }
        Ok(total_bytes_written)
    }
}
