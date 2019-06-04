use auto_struct_macros::auto_struct;
use reader_writer::{DiffList, DiffListSourceCursor, AsDiffListSourceCursor, FourCC, Readable,
                    Reader, RoArray, Writable,
                    align_byte_count};


use std::io;
use std::borrow::Cow;

use crate::{
    mlvl::Mlvl,
    mrea::Mrea,
    savw::Savw,
    hint::Hint,
    strg::Strg,
    scan::Scan,
};

#[auto_struct(Readable, Writable)]
#[derive(Clone, Debug)]
pub struct Pak<'r>
{
    pub start: Reader<'r>,
    #[auto_struct(expect = 0x00030005)]
    version: u32,
    pub unused: u32,

    #[auto_struct(derive = named_resources.len() as u32)]
    named_resources_count: u32,
    #[auto_struct(init = (named_resources_count as usize, ()))]
    pub named_resources: RoArray<'r, NamedResource<'r>>,

    #[auto_struct(derive = resources.len() as u32)]
    resources_count: u32,

    #[auto_struct(derive_from_iter = {
            let starting_offset = align_byte_count(32,
                    named_resources.size() +
                    <u32 as Readable>::fixed_size().unwrap() * 4 +
                    <ResourceInfo as Readable>::fixed_size().unwrap() * resources.len()
                ) as u32;
            resources.iter().scan(starting_offset, |offset, res| {
                let info = res.resource_info(*offset);
                *offset += info.size;
                Some(info)
            })
        })]
    #[auto_struct(init = (resources_count as usize, ()))]
    resource_info: RoArray<'r, ResourceInfo>,

    #[auto_struct(pad_align = 32)]
    _pad: (),

    #[auto_struct(init = ResourceSource(start.clone(), resource_info.clone()))]
    pub resources: DiffList<ResourceSource<'r>>,


    #[auto_struct(pad_align = 32)]
    _pad: (),
}


#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct NamedResource<'r>
{
    pub fourcc: FourCC,
    pub file_id: u32,
    pub name_length: u32,
    #[auto_struct(init = (name_length as usize, ()))]
    pub name: RoArray<'r, u8>,
}


#[auto_struct(Readable, FixedSize, Writable)]
#[derive(Debug, Clone, Copy)]
pub struct ResourceInfo
{
    pub compressed: u32,
    pub fourcc: FourCC,
    pub file_id: u32,
    pub size: u32,
    pub offset: u32,
}


#[derive(Debug, Clone)]
pub struct ResourceSource<'r>(Reader<'r>, RoArray<'r, ResourceInfo>);
#[derive(Debug, Clone)]
pub struct ResourceSourceCursor<'r>
{
    pak_start: Reader<'r>,
    info_array: RoArray<'r, ResourceInfo>,
    index: usize,
}

impl<'r> AsDiffListSourceCursor for ResourceSource<'r>
{
    type Cursor = ResourceSourceCursor<'r>;
    fn as_cursor(&self) -> Self::Cursor
    {
        ResourceSourceCursor {
            pak_start: self.0.clone(),
            info_array: self.1.clone(),
            index: 0,
        }
    }

    fn len(&self) -> usize
    {
        self.1.len()
    }
}

impl<'r> DiffListSourceCursor for ResourceSourceCursor<'r>
{
    type Item = Resource<'r>;
    type Source = ResourceSource<'r>;
    fn next(&mut self) -> bool
    {
        if self.index == self.info_array.len() - 1 {
            false
        } else {
            self.index += 1;
            true
        }
    }

    fn get(&self) -> Self::Item
    {
        let info = self.info_array.get(self.index).unwrap();
        self.pak_start.offset(info.offset as usize).read(info.clone())
    }

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


#[derive(Debug, Clone)]
pub struct Resource<'r>
{
    pub compressed: bool,
    pub file_id: u32,
    pub kind: ResourceKind<'r>,
    #[cfg(debug_assertions)]
    pub original_offset: u32,
}

impl<'r> Resource<'r>
{
    pub fn resource_info(&self, offset: u32) -> ResourceInfo
    {
        ResourceInfo {
            compressed: self.compressed as u32,
            fourcc: self.fourcc(),
            file_id: self.file_id,
            size: self.size() as u32,
            offset: offset,
        }
    }

    pub fn fourcc(&self) -> FourCC
    {
        self.kind.fourcc()
    }
}

impl<'r> Readable<'r> for Resource<'r>
{
    type Args = ResourceInfo;
    #[cfg(debug_assertions)]
    fn read_from(reader: &mut Reader<'r>, info: Self::Args) -> Self
    {
        if info.compressed > 1 {
            panic!("Bad info.compressed")
        };
        let res = Resource {
            compressed: info.compressed == 1,
            file_id: info.file_id,
            kind: ResourceKind::Unknown(reader.truncated(info.size as usize), info.fourcc),
            original_offset: info.offset,
        };
        reader.advance(info.size as usize);
        res
    }
    #[cfg(not(debug_assertions))]
    fn read_from(reader: &mut Reader<'r>, info: Self::Args) -> Self
    {
        let res = Resource {
            compressed: info.compressed == 1,
            file_id: info.file_id,
            kind: ResourceKind::Unknown(reader.truncated(info.size as usize), info.fourcc),
        };
        reader.advance(info.size as usize);
        res
    }

    fn size(&self) -> usize
    {
        self.kind.size()
    }
}

impl<'r> Writable for Resource<'r>
{
    fn write_to<W: io::Write>(&self, writer: &mut W) -> io::Result<u64>
    {
        self.kind.write_to(writer)
    }
}

macro_rules! build_resource_data {
    ($($name:ident, $fourcc:expr, $accessor:ident, $accessor_mut:ident,)*) => {

        #[derive(Clone, Debug)]
        pub enum ResourceKind<'r>
        {
            Unknown(Reader<'r>, FourCC),
            External(Vec<u8>, FourCC),
            $($name($name<'r>),)*
        }

        impl<'r> ResourceKind<'r>
        {
            pub fn fourcc(&self) -> FourCC
            {
                match *self {
                    ResourceKind::Unknown(_, fourcc) => fourcc,
                    ResourceKind::External(_, fourcc) => fourcc,
                    $(ResourceKind::$name(_) => $fourcc.into(),)*
                }
            }

            pub fn guess_kind(&mut self)
            {
                let (mut reader, fourcc) = match *self {
                    ResourceKind::Unknown(ref reader, fourcc) => (reader.clone(), fourcc),
                    _ => return,
                };

                if false { }
                $(else if fourcc == $fourcc.into() {
                    *self = ResourceKind::$name(reader.read(()));
                })*
            }

            $(
                pub fn $accessor(&self) -> Option<Cow<$name<'r>>>
                {
                    match *self {
                        ResourceKind::$name(ref inst) => Some(Cow::Borrowed(inst)),
                        ResourceKind::Unknown(ref reader, fourcc) => {
                            if fourcc == $fourcc.into() {
                                Some(Cow::Owned(reader.clone().read(())))
                            } else {
                                None
                            }
                        },
                        _ => None,
                    }
                }

                pub fn $accessor_mut(&mut self) -> Option<&mut $name<'r>>
                {
                    self.guess_kind();
                    match *self {
                        ResourceKind::$name(ref mut inst) => Some(inst),
                        _ => None,
                    }
                }
            )*

            fn size(&self) -> usize
            {
                match *self {
                    ResourceKind::Unknown(ref data, _) => data.len(),
                    ResourceKind::External(ref data, _) => data.len(),
                    $(ResourceKind::$name(ref i) => i.size(),)*
                }
            }

            fn write_to<W: io::Write>(&self, writer: &mut W) -> io::Result<u64>
            {
                match *self {
                    ResourceKind::Unknown(ref data, _) => {
                        writer.write_all(&data)?;
                        Ok(data.len() as u64)
                    },
                    ResourceKind::External(ref data, _) => {
                        writer.write_all(&data)?;
                        Ok(data.len() as u64)
                    },
                    $(ResourceKind::$name(ref i) => i.write_to(writer),)*
                }
            }
        }
    };
}

build_resource_data!(
    Mrea, b"MREA", as_mrea, as_mrea_mut,
    Mlvl, b"MLVL", as_mlvl, as_mlvl_mut,
    Savw, b"SAVW", as_savw, as_savw_mut,
    Hint, b"HINT", as_hint, as_hint_mut,
    Strg, b"STRG", as_strg, as_strg_mut,
    Scan, b"SCAN", as_scan, as_scan_mut,
);

