use reader_writer::{DiffList, DiffListSourceCursor, AsDiffListSourceCursor, FourCC, Readable,
                    Reader, RoArray, Writable,
                    align_byte_count, pad_bytes_count, pad_bytes, pad_bytes_ff};


use std::io::Write;

use mlvl::Mlvl;
use mrea::Mrea;
use savw::Savw;

auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Clone, Debug)]
    pub struct Pak<'a>
    {
        start: Reader<'a>,
        #[expect = 0x00030005]
        version: u32,
        unused: u32,

        #[derivable = named_resources.len() as u32]
        named_resources_count: u32,
        named_resources: RoArray<'a, NamedResource<'a>> = (named_resources_count as usize, ()),

        #[derivable = resources.len() as u32]
        resources_count: u32,

        #[derivable: ResourceInfoProxy = ResourceInfoProxy(&resources, named_resources.size())]
        resource_info: RoArray<'a, ResourceInfo> = (resources_count as usize, ()),

        #[offset]
        offset: usize,
        #[derivable = pad_bytes(32, offset)]
        _padding: RoArray<'a, u8> = (pad_bytes_count(32, offset), ()),

        resources: DiffList<'a, ResourceSource<'a>> = ResourceSource(start.clone(),
                                                                     resource_info.clone()),

        #[offset]
        offset_after: usize,
        #[derivable = pad_bytes_ff(32, offset_after)]
        _padding_after: RoArray<'a, u8> = (pad_bytes_count(32, offset_after), ()),
    }
}


auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Debug, Clone)]
    pub struct NamedResource<'a>
    {
        fourcc: FourCC,
        file_id: u32,
        name_length: u32,
        name: RoArray<'a, u8> = (name_length as usize, ()),
    }
}


auto_struct! {
    #[auto_struct(Readable, FixedSize, Writable)]
    #[derive(Debug, Clone, Copy)]
    pub struct ResourceInfo
    {
        compressed: u32,
        fourcc: FourCC,
        file_id: u32,
        size: u32,
        offset: u32,
    }
}


#[derive(Debug, Clone)]
pub struct ResourceSource<'a>(Reader<'a>, RoArray<'a, ResourceInfo>);
#[derive(Debug, Clone)]
pub struct ResourceSourceCursor<'a>
{
    pak_start: Reader<'a>,
    info_array: RoArray<'a, ResourceInfo>,
    index: usize,
}

impl<'a> AsDiffListSourceCursor for ResourceSource<'a>
{
    type Cursor = ResourceSourceCursor<'a>;
    #[inline]
    fn as_cursor(&self) -> Self::Cursor
    {
        ResourceSourceCursor {
            pak_start: self.0.clone(),
            info_array: self.1.clone(),
            index: 0,
        }
    }

    #[inline]
    fn len(&self) -> usize
    {
        self.1.len()
    }
}

impl<'a> DiffListSourceCursor for ResourceSourceCursor<'a>
{
    type Item = Resource<'a>;
    type Source = ResourceSource<'a>;
    #[inline]
    fn next(&mut self) -> bool
    {
        if self.index == self.info_array.len() - 1 {
            false
        } else {
            self.index += 1;
            true
        }
    }

    #[inline]
    fn get(&self) -> Self::Item
    {
        let info = self.info_array.get(self.index).unwrap();
        self.pak_start.offset(info.offset as usize).read(info.clone())
    }

    #[inline]
    fn split(mut self) -> (Option<Self::Source>, Self::Source)
    {
        let pak_start = self.pak_start;
        let f = |a| ResourceSource(pak_start.clone(), a);
        if self.index == 0 {
            (None, f(self.info_array))
        } else {
            let left = self.info_array.split_off(self.index);
            (Some(f(left)), f(self.info_array))
        }
   }

    #[inline]
    fn split_around(mut self) -> (Option<Self::Source>, Self::Item, Option<Self::Source>)
    {
        let item = self.get();
        let pak_start = self.pak_start;
        let f = |a| Some(ResourceSource(pak_start.clone(), a));
        if self.info_array.len() == 1 {
            (None, item, None)
        } else if self.index == 0 {
            let right = self.info_array.split_off(1);
            (None, item, f(right))
        } else if self.index == self.info_array.len() - 1 {
            let _ = self.info_array.split_off(self.index);
            (f(self.info_array), item, None)
        } else {
            let mut right = self.info_array.split_off(self.index);
            let right = right.split_off(1);
            (f(self.info_array), item, f(right))
        }
    }
}

struct ResourceInfoProxy<'a, 'list>(&'list DiffList<'a, ResourceSource<'a>>, usize)
    where 'a: 'list;
impl<'a, 'list> Readable<'a> for ResourceInfoProxy<'a, 'list>
    where 'a: 'list
{
    type Args = ();
    fn read(_: Reader<'a>, (): ()) -> (Self, Reader<'a>)
    {
        panic!("This proxy shouldn't be read.")
    }

    #[inline]
    fn size(&self) -> usize
    {
        ResourceInfo::fixed_size().unwrap() * self.0.len()
    }
}

impl<'a, 'list> Writable for ResourceInfoProxy<'a, 'list>
    where 'a: 'list
{
    fn write<W: Write>(&self, writer: &mut W)
    {
        let mut offset = align_byte_count(32,
                self.1 +
                u32::fixed_size().unwrap() * 4 +
                ResourceInfo::fixed_size().unwrap() * self.0.len()
            ) as u32;
        for res in self.0.iter() {
            let info = res.resource_info(offset);
            info.write(writer);
            offset += info.size;
        }
    }
}

#[derive(Debug, Clone)]
pub struct Resource<'a>
{
    pub compressed: bool,
    pub fourcc: FourCC,
    pub file_id: u32,
    pub kind: ResourceKind<'a>,
    #[cfg(debug_assertions)]
    pub original_offset: u32,
}

impl<'a> Resource<'a>
{
    pub fn resource_info(&self, offset: u32) -> ResourceInfo
    {
        ResourceInfo {
            compressed: self.compressed as u32,
            fourcc: self.fourcc,
            file_id: self.file_id,
            size: self.size() as u32,
            offset: offset,
        }
    }

    pub fn guess_kind(&mut self)
    {
        let reader = match self.kind {
            ResourceKind::Unknown(ref reader) => reader.clone(),
            _ => return,
        };
        if self.fourcc == FourCC::from_bytes(b"MREA") {
            self.kind = ResourceKind::Mrea(reader.clone().read(()));
        } else if self.fourcc == FourCC::from_bytes(b"MLVL") {
            self.kind = ResourceKind::Mlvl(reader.clone().read(()));
        } else if self.fourcc == FourCC::from_bytes(b"SAVW") {
            self.kind = ResourceKind::Savw(reader.clone().read(()));
        }
    }
}

impl<'a> Readable<'a> for Resource<'a>
{
    type Args = ResourceInfo;
    #[cfg(debug_assertions)]
    fn read(reader: Reader<'a>, info: Self::Args) -> (Self, Reader<'a>)
    {
        if info.compressed > 1 {
            panic!("Bad info.compressed")
        };
        let res = Resource {
            compressed: info.compressed == 1,
            fourcc: info.fourcc,
            file_id: info.file_id,
            kind: ResourceKind::Unknown(reader.truncated(info.size as usize)),
            original_offset: info.offset,
        };
        (res, reader.offset(info.size as usize))
    }
    #[cfg(not(debug_assertions))]
    fn read(reader: Reader<'a>, info: Self::Args) -> (Self, Reader<'a>)
    {
        let res = Resource {
            compressed: info.compressed == 1,
            fourcc: info.fourcc,
            file_id: info.file_id,
            kind: ResourceKind::Unknown(reader.truncated(info.size as usize)),
        };
        (res, reader.offset(info.size as usize))
    }

    fn size(&self) -> usize
    {
        match self.kind {
            ResourceKind::Unknown(ref reader) => reader.len(),
            ResourceKind::Mrea(ref mrea) => mrea.size(),
            ResourceKind::Mlvl(ref mlvl) => mlvl.size(),
            ResourceKind::Savw(ref savw) => savw.size(),
        }
    }
}

impl<'a> Writable for Resource<'a>
{
    fn write<W: Write>(&self, writer: &mut W)
    {
        match self.kind {
            ResourceKind::Unknown(ref reader) => writer.write_all(&reader).unwrap(),
            ResourceKind::Mrea(ref mrea) => mrea.write(writer),
            ResourceKind::Mlvl(ref mlvl) => mlvl.write(writer),
            ResourceKind::Savw(ref savw) => savw.write(writer),
        }
    }
}

#[derive(Clone, Debug)]
pub enum ResourceKind<'a>
{
    Unknown(Reader<'a>),
    Mrea(Mrea<'a>),
    Mlvl(Mlvl<'a>),
    Savw(Savw<'a>),
    //UnknownCompressed(Reader<'a>),
}

