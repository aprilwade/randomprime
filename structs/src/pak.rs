use auto_struct_macros::auto_struct;
use reader_writer::{
    FourCC, LCow, Readable, Reader, RoArray, Writable, align_byte_count, pad_bytes,
};


use std::borrow::Cow;
use std::fmt;
use std::io;
use std::iter;
use std::ops;

use crate::{
    evnt::Evnt,
    frme::Frme,
    hint::Hint,
    mapa::Mapa,
    mapw::Mapw,
    mlvl::Mlvl,
    mrea::Mrea,
    savw::Savw,
    scan::Scan,
    strg::Strg,
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

    #[auto_struct(init = (start.clone(), resource_info))]
    pub resources: ResourceList<'r>,


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

impl ResourceInfo
{
    fn get_resource<'r>(&self, pak_start: Reader<'r>) -> Resource<'r>
    {
        pak_start.offset(self.offset as usize).read(*self)
    }
}

#[derive(Clone, Debug)]
enum ResourceListElem<'r>
{
    Array(RoArray<'r, ResourceInfo>),
    Inst(Resource<'r>),
}

impl<'r> ResourceListElem<'r>
{
    fn len(&self) -> usize
    {
        match *self {
            ResourceListElem::Array(ref array) => array.len(),
            ResourceListElem::Inst(_) => 1,
        }
    }
}

#[derive(Clone)]
pub struct ResourceList<'r>
{
    pak_start: Option<Reader<'r>>,
    list: Vec<ResourceListElem<'r>>,
}

impl<'r> ResourceList<'r>
{
    pub fn cursor<'s>(&'s mut self) -> ResourceListCursor<'r, 's>
    {
        let inner_cursor = match self.list.get(0) {
                Some(ResourceListElem::Array(a)) => Some(InnerCursor {
                    info_array: a.clone(),
                    idx: 0
                }),
                _ => None,
            };
        ResourceListCursor {
            list: self,
            idx: 0,
            inner_cursor,
        }

    }

    pub fn iter<'s>(&'s self) -> ResourceListIter<'r, 's>
    {
        ResourceListIter {
            pak_start: self.pak_start.as_ref(),
            list_iter: self.list.iter(),
            inner_cursor: None,
        }
    }

    pub fn len(&self) -> usize
    {
        // TODO: It might make sense to cache this...
        self.list.iter().map(|elem| elem.len()).sum()
    }

    pub fn clear(&mut self)
    {
        self.list.clear()
    }
}

impl<'r> Readable<'r> for ResourceList<'r>
{
    type Args = (Reader<'r>, RoArray<'r, ResourceInfo>);
    fn read_from(reader: &mut Reader<'r>, (pak_start, info_array): Self::Args) -> Self
    {
        let res = ResourceList {
            pak_start: Some(pak_start),
            list: vec![ResourceListElem::Array(info_array)],
        };
        reader.advance(res.size());
        res
    }

    fn size(&self) -> usize
    {
        self.iter().fold(0, |s, i| s + i.size())
    }
}

impl<'r> Writable for ResourceList<'r>
{
    fn write_to<W: io::Write>(&self, writer: &mut W) -> io::Result<u64>
    {
        let mut s = 0;
        for i in self.iter() {
            s += i.write_to(writer)?
        }
        Ok(s)
    }
}

impl<'r> fmt::Debug for ResourceList<'r>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error>
    {
        write!(f, "ResourceList {{ list: {:?} }}", self.list)
    }
}

impl<'r> iter::FromIterator<Resource<'r>> for ResourceList<'r>
{
    fn from_iter<I>(i: I) -> Self
        where I: IntoIterator<Item = Resource<'r>>
    {
        ResourceList {
            pak_start: None,
            list: i.into_iter().map(|x| ResourceListElem::Inst(x)).collect(),
        }
    }
}

#[derive(Clone)]
struct InnerCursor<'r>
{
    info_array: RoArray<'r, ResourceInfo>,
    idx: usize,
}


impl<'r> InnerCursor<'r>
{
    fn next(&mut self) -> bool
    {
        if self.idx == self.info_array.len() - 1 {
            false
        } else {
            self.idx += 1;
            true
        }
    }

    fn get(&self) -> ResourceInfo
    {
        self.info_array.get(self.idx).unwrap()
    }

    fn split(mut self) -> (Option<RoArray<'r, ResourceInfo>>, RoArray<'r, ResourceInfo>)
    {
        if self.idx == 0 {
            (None, self.info_array)
        } else {
            let left = self.info_array.split_off(self.idx);
            (Some(left), self.info_array)
        }
   }

   fn split_around(mut self)
        -> (Option<RoArray<'r, ResourceInfo>>, ResourceInfo, Option<RoArray<'r, ResourceInfo>>)
    {
        let item = self.get();
        if self.info_array.len() == 1 {
            (None, item, None)
        } else if self.idx == 0 {
            let right = self.info_array.split_off(1);
            (None, item, Some(right))
        } else if self.idx == self.info_array.len() - 1 {
            let _ = self.info_array.split_off(self.idx);
            (Some(self.info_array), item, None)
        } else {
            let mut right = self.info_array.split_off(self.idx);
            let right = right.split_off(1);
            (Some(self.info_array), item, Some(right))
        }
    }
}

pub struct ResourceListCursor<'r, 'list>
{
    list: &'list mut ResourceList<'r>,
    idx: usize,
    inner_cursor: Option<InnerCursor<'r>>,
}

impl<'r, 'list> ResourceListCursor<'r, 'list>
{
    // TODO: Return value?
    pub fn next(&mut self)
    {
        let advance_cursor = self.inner_cursor.as_mut().map(|ic| !ic.next()).unwrap_or(true);
        if advance_cursor && !self.list.list.get(self.idx).is_none() {
            self.inner_cursor = None;
            self.idx += 1;
            match self.list.list.get(self.idx) {
                None => (),
                Some(ResourceListElem::Inst(_)) => (),
                Some(ResourceListElem::Array(a)) => {
                    self.inner_cursor = Some(InnerCursor {
                        info_array: a.clone(),
                        idx: 0,
                    });
                },
            };
        };
    }

    // TODO: prev?

    /// Inserts the items yielded by `iter` into the list. The cursor will be
    /// positioned at the first inserted item.
    pub fn insert_before<I>(&mut self, iter: I)
        where I: Iterator<Item = Resource<'r>>
    {
        let mut iter = iter.peekable();
        if iter.peek().is_none() {
            return;
        };

        // XXX This could probably be made more efficent by combining the insert with the splice,
        //     but it'd probably be even harder to understand...
        if let Some(ic) = self.inner_cursor.take() {
            let (left, right) = ic.split();
            if let Some(left) = left {
                self.list.list.insert(self.idx, ResourceListElem::Array(left));
                self.idx += 1
            };
            self.list.list[self.idx] = ResourceListElem::Array(right);
        };
        self.list.list.splice(self.idx..self.idx, iter.map(ResourceListElem::Inst));
        // We shouldn't need to set self.inner_cursor here. We've inserted at
        // least one element, so self.cursor should be pointing to an Inst.
    }

    /// Inserts the items yielded by `iter` into the list. The cursor will be positioned after the
    /// last inserted item (the same item it was originally pointed to).
    pub fn insert_after<I>(&mut self, iter: I)
        where I: Iterator<Item = Resource<'r>>
    {
        let mut iter = iter.peekable();
        if iter.peek().is_none() {
            return;
        };

        // XXX This could probably be made more efficent by combining the insert with the splice,
        //     but it'd probably be even harder to understand...
        let pre_len = self.list.list.len();
        if let Some(ic) = self.inner_cursor.take() {
            let (left, right) = ic.split();
            if let Some(left) = left {
                self.list.list.insert(self.idx, ResourceListElem::Array(left));
                self.idx += 1
            };
            self.list.list[self.idx] = ResourceListElem::Array(right);
        };
        self.list.list.splice(self.idx..self.idx, iter.map(ResourceListElem::Inst));
        self.idx += self.list.list.len() - pre_len
        // We shouldn't need to set self.inner_cursor here. We've inserted at
        // least one element, so self.cursor should be pointing to an Inst.
    }

    pub fn peek(&mut self) -> Option<LCow<Resource<'r>>>
    {
        if let Some(ref ic) = self.inner_cursor {
            Some(LCow::Owned(ic.get().get_resource(self.list.pak_start.as_ref().unwrap().clone())))
        } else {
            match self.list.list.get(self.idx) {
                None => None,
                Some(ResourceListElem::Array(_)) => unreachable!(),
                Some(ResourceListElem::Inst(res)) => Some(LCow::Borrowed(res)),
            }
        }
    }

    pub fn value(&mut self) -> Option<&mut Resource<'r>>
    {
        if let Some(ic) = self.inner_cursor.take() {
            let (left, info, right) = ic.split_around();
            let elem = info.get_resource(self.list.pak_start.as_ref().unwrap().clone());
            if let Some(right) = right {
                // There are elements to the right
                self.list.list[self.idx] = ResourceListElem::Array(right);
                self.list.list.insert(self.idx, ResourceListElem::Inst(elem));
            } else {
                // This was the last element.
                self.list.list[self.idx] = ResourceListElem::Inst(elem);
            };
            // self.cursor now points to the correct Inst
            if let Some(left) = left {
                // There are elements to the left.
                self.list.list.insert(self.idx, ResourceListElem::Array(left));
                self.idx += 1
            };
        };
        match self.list.list.get_mut(self.idx) {
            Some(&mut ResourceListElem::Inst(ref mut inst)) => Some(inst),
            Some(&mut ResourceListElem::Array(_)) => unreachable!(),
            None => None,
        }
    }

    pub fn into_value(mut self) -> Option<&'list mut Resource<'r>>
    {
        self.value();
        match self.list.list.get_mut(self.idx) {
            Some(&mut ResourceListElem::Inst(ref mut inst)) => Some(inst),
            Some(&mut ResourceListElem::Array(_)) => unreachable!(),
            None => None,
        }
    }

    pub fn cursor_advancer<'s>(&'s mut self) -> ResourceListCursorAdvancer<'r, 'list, 's>
    {
        ResourceListCursorAdvancer { cursor: self }
    }
}


/// Wraps a ResourceListCursor and automatically advances it when it is dropped.
pub struct ResourceListCursorAdvancer<'r, 'list, 'cursor>
{
    cursor: &'cursor mut ResourceListCursor<'r, 'list>,
}

impl<'r, 'list, 'cursor> Drop for ResourceListCursorAdvancer<'r, 'list, 'cursor>
{
    fn drop(&mut self)
    {
        self.cursor.next()
    }
}

impl<'r, 'list, 'cursor> ops::Deref for ResourceListCursorAdvancer<'r, 'list, 'cursor>
{
    type Target = ResourceListCursor<'r, 'list>;
    fn deref(&self) -> &Self::Target
    {
        &*self.cursor
    }
}

impl<'r, 'list, 'cursor> ops::DerefMut for ResourceListCursorAdvancer<'r, 'list, 'cursor>
{
    fn deref_mut(&mut self) -> &mut Self::Target
    {
        &mut *self.cursor
    }
}

#[derive(Clone)]
pub struct ResourceListIter<'r, 'list>
{
    pak_start: Option<&'list Reader<'r>>,
    list_iter: std::slice::Iter<'list, ResourceListElem<'r>>,
    inner_cursor: Option<InnerCursor<'r>>,
}

impl<'r, 'list> Iterator for ResourceListIter<'r, 'list>
{
    type Item = LCow<'list, Resource<'r>>;
    fn next(&mut self) -> Option<Self::Item>
    {
        if let Some(cursor) = &mut self.inner_cursor {
            if cursor.next() {
                return Some(LCow::Owned(cursor.get().get_resource(self.pak_start.unwrap().clone())))
            }
        }
        match self.list_iter.next() {
            Some(ResourceListElem::Array(info_array)) => {
                let cursor = InnerCursor {
                    info_array: info_array.clone(),
                    idx: 0,
                };
                let res = cursor.get().get_resource(self.pak_start.unwrap().clone());
                self.inner_cursor = Some(cursor);
                Some(LCow::Owned(res))
            },
            Some(ResourceListElem::Inst(inst)) => Some(LCow::Borrowed(inst)),
            None => None,
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
            offset,
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
        align_byte_count(32, self.kind.size())
    }
}

impl<'r> Writable for Resource<'r>
{
    fn write_to<W: io::Write>(&self, writer: &mut W) -> io::Result<u64>
    {
        let bytes_written = self.kind.write_to(writer)?;
        let padding_written = pad_bytes(32, bytes_written as usize).write_to(writer)?;
        Ok(bytes_written + padding_written)
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
                let (mut reader, fourcc) = match self {
                    ResourceKind::Unknown(reader, fourcc) => (reader.clone(), *fourcc),
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
                    match self {
                        ResourceKind::$name(inst) => Some(Cow::Borrowed(inst)),
                        ResourceKind::Unknown(reader, fourcc) => {
                            if *fourcc == $fourcc.into() {
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
    Evnt, b"EVNT", as_evnt, as_evnt_mut,
    Frme, b"FRME", as_frme, as_frme_mut,
    Hint, b"HINT", as_hint, as_hint_mut,
    Mapa, b"MAPA", as_mapa, as_mapa_mut,
    Mapw, b"MAPW", as_mapw, as_mapw_mut,
    Mlvl, b"MLVL", as_mlvl, as_mlvl_mut,
    Mrea, b"MREA", as_mrea, as_mrea_mut,
    Savw, b"SAVW", as_savw, as_savw_mut,
    Scan, b"SCAN", as_scan, as_scan_mut,
    Strg, b"STRG", as_strg, as_strg_mut,
);

