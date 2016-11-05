use reader_writer::{Array, ArrayIterator, Dap, FourCC, ImmCow, Readable, Reader, Writable,
                    pad_bytes_count, pad_bytes, pad_bytes_ff};

use linked_list::{Cursor as LinkedListCursor, Iter as LinkedListIter, LinkedList};

use std::mem;
use std::iter;
use std::iter::FromIterator;
use std::borrow::Borrow;
use std::io::Write;

use mrea::Mrea;

auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Debug)]
    pub struct Pak<'a>
    {
        start: Reader<'a>,
        #[expect = 0x00030005]
        version: u32,
        unused: u32,

        #[derivable = named_resources.len() as u32]
        named_resources_count: u32,
        named_resources: Array<'a, NamedResource<'a>> = (named_resources_count as usize, ()),

        #[derivable = resources.len() as u32]
        resources_count: u32,
        #[derivable: Dap<_, _> = &resources.info_iter(named_resources.size()).into()]
        resource_info: Array<'a, ResourceInfo> = (resources_count as usize, ()),

        #[offset]
        offset: usize,
        #[derivable = pad_bytes(32, offset)]
        _padding: Array<'a, u8> = (pad_bytes_count(32, offset), ()),

        resources: ResourceTable<'a> = (start.clone(), resource_info),

        #[offset]
        offset_after: usize,
        #[derivable = pad_bytes_ff(32, offset_after)]
        _padding_after: Array<'a, u8> = (pad_bytes_count(32, offset_after), ()),
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
        name: Array<'a, u8> = (name_length as usize, ()),
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

fn make_resource<'a, B: Borrow<ResourceInfo>>(info: B, pak_start: &Reader<'a>) -> Resource_<'a>
{
    pak_start.offset(info.borrow().offset as usize).read(info.borrow().clone())
}

auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Debug, Clone)]
    pub struct Resource<'a>
    {
        #[args]
        info: ResourceInfo,

        #[literal]
        info: ResourceInfo = info,
        data: Array<'a, u8> = (info.size as usize, ()),
    }
}

#[derive(Debug)]
pub struct ResourceTable<'a>
{
    data_start: Reader<'a>,
    list: LinkedList<DiffListElem<'a>>,
}

impl<'a> ResourceTable<'a>
{
    pub fn cursor<'s>(&'s mut self) -> ResourceTableCursor<'a, 's>
    {
        ResourceTableCursor {
            cursor: self.list.cursor(),
            index: 0,
            reader: self.data_start.clone(),
        }
    }

    pub fn iter<'s>(&'s self) -> ResourceTableIterator<'a, 's>
    {
        ResourceTableIterator {
            list_iter: self.list.iter(),
            info_iter: None,
            pak_start: self.data_start.clone()
        }
    }

    pub fn info_iter<'s>(&'s self, named_resources_size: usize) -> ResourceTableInfoIter<'a, 's>
    {
        let offset = 4 * u32::fixed_size().unwrap() + named_resources_size +
                    ResourceInfo::fixed_size().unwrap() * self.len();
        let offset = (offset + 31) & (usize::max_value() - 31);
        ResourceTableInfoIter {
            iter: self.iter(),
            offset: offset,
        }
    }

    pub fn len(&self) -> usize
    {
        // TODO: It might make sense to cache this...
        self.list.iter().map(|elem| elem.len()).sum()
    }
}

type InfoArray<'a> = Array<'a, ResourceInfo>;
impl<'a> Readable<'a> for ResourceTable<'a>
{
    type Args = (Reader<'a>, InfoArray<'a>);
    fn read(reader: Reader<'a>, (pak_start, info): Self::Args) -> (Self, Reader<'a>)
    {
        use std::iter::FromIterator;
        let table = ResourceTable {
            data_start: pak_start,
            list: LinkedList::from_iter(iter::once(DiffListElem::Array(info))),
        };
        let reader = reader.offset(table.size());
        (table, reader)
    }

    fn size(&self) -> usize
    {
        self.iter().map(|res| res.size()).sum()
    }
}

impl<'a> Writable for ResourceTable<'a>
{
    fn write<W: Write>(&self, writer: &mut W)
    {
        for elem in self.list.iter() {
            match *elem {
                DiffListElem::Array(ref array) => {
                    let start = array.get(0).unwrap().offset as usize;
                    let final_info = array.get(array.len() - 1).unwrap();
                    let end = (final_info.offset + final_info.size) as usize;
                    writer.write_all(&(*self.data_start)[start..end]).unwrap();
                },
                DiffListElem::Inst(ref inst) => inst.write(writer),
            }
        }
    }
}


pub struct ResourceTableCursor<'a, 'list>
    where 'a: 'list,
{
    cursor: LinkedListCursor<'list, DiffListElem<'a>>,
    index: usize,
    reader: Reader<'a>,
}

impl<'a, 'list> ResourceTableCursor<'a, 'list>
    where 'a: 'list
{
    // TODO: Return value?
    pub fn next(&mut self)
    {
        let advance_cursor = match self.cursor.peek_next() {
            Some(&mut DiffListElem::Array(ref info)) => {
                if self.index == info.len() - 1 {
                    self.index = 0;
                    true
                } else {
                    self.index += 1;
                    false
                }
            },
            Some(&mut DiffListElem::Inst(_)) => {
                self.index = 0;
                true
            },
            None => false,
        };
        if advance_cursor {
            self.cursor.next();
        };
    }

    // TODO: prev?

    /// Inserts the items yielded by `iter` into the list. The cursor will be
    /// positioned at the first inserted item.
    pub fn insert_before<I>(&mut self, iter: I)
        where I: Iterator<Item = Resource_<'a>>
    {
        // If we're sitting inside an array, split it
        let before = match self.cursor.peek_next() {
            Some(&mut DiffListElem::Array(ref mut info_array)) => {
                if self.index == 0 {
                    None
                } else if self.index == info_array.len() - 1 {
                    let info = info_array.get(self.index).unwrap().clone();
                    info_array.split_off(self.index);
                    Some(DiffListElem::Inst(make_resource(info, &self.reader)))
                } else {
                    let mut after = info_array.split_off(self.index);
                    mem::swap(&mut after, info_array);
                    Some(DiffListElem::Array(after))
                }
            },
            _ => None,
        };
        if let Some(before) = before {
            self.cursor.insert(before);
        };
        self.cursor.splice(&mut LinkedList::from_iter(iter.map(|i| DiffListElem::Inst(i))));
    }

    pub fn peek(&mut self) -> Option<ImmCow<Resource_<'a>>>
    {
        match self.cursor.peek_next() {
            Some(&mut DiffListElem::Array(ref info)) => {
                let info = info.get(self.index).unwrap();
                Some(ImmCow::new_owned(make_resource(info, &self.reader)))
            },
            Some(&mut DiffListElem::Inst(ref res)) => Some(ImmCow::new_borrowed(res)),
            None => None,
        }
    }

    // XXX This potentially allocates memory; use with caution
    pub fn value(&mut self) -> Option<&mut Resource_<'a>>
    {
        // This looks a little ridiculous; its a silly hack to acheive pseudo-gotos...
        loop {
            let mut info_array = match self.cursor.peek_next() {
                Some(&mut DiffListElem::Array(ref info)) => info.clone(),
                Some(_) => break,
                None => return None,
            };

            // 4 cases
            if info_array.len() == 1 {
                let info = info_array.get(0).unwrap();
                *self.cursor.peek_next().unwrap() = DiffListElem::Inst(
                    make_resource(info, &self.reader));
            } else if self.index == 0 {
                let info = info_array.get(0).unwrap().clone();
                let remaining_info = info_array.split_off(1);
                *self.cursor.peek_next().unwrap() = DiffListElem::Array(remaining_info);
                self.cursor.insert(DiffListElem::Inst(make_resource(info, &self.reader)));
            } else if self.index == info_array.len() - 1 {
                let info = info_array.get(self.index).unwrap().clone();

                info_array.split_off(self.index);
                *self.cursor.peek_next().unwrap() = DiffListElem::Inst(
                    make_resource(info, &self.reader));

                self.cursor.insert(DiffListElem::Array(info_array));

                self.cursor.next();
            } else {
                let info = info_array.get(self.index).unwrap().clone();
                let info_after = info_array.split_off(self.index).split_off(1);
                let info_before = info_array;
                
                *self.cursor.peek_next().unwrap() = DiffListElem::Array(info_after);
                self.cursor.insert(DiffListElem::Inst(make_resource(info, &self.reader)));
                self.cursor.insert(DiffListElem::Array(info_before));

                self.cursor.next();
            }
            break;
        }

        match self.cursor.peek_next() {
            Some(&mut DiffListElem::Inst(ref mut res)) => Some(res),
            _ => unreachable!(),
        }
    }
}

#[derive(Clone)]
pub struct ResourceTableIterator<'a, 'list>
    where 'a: 'list
{
    list_iter: LinkedListIter<'list, DiffListElem<'a>>,
    info_iter: Option<ArrayIterator<'list, 'a, ResourceInfo>>,
    pak_start: Reader<'a>,
}

impl<'a, 'list> Iterator for ResourceTableIterator<'a, 'list>
    where 'a: 'list
{
    type Item = ImmCow<'list, Resource_<'a>>;
    fn next(&mut self) -> Option<Self::Item>
    {
        loop {
            if let Some(ref mut iter) = self.info_iter {
                if let Some(info) = iter.next() {
                    return Some(ImmCow::new_owned(make_resource(info, &self.pak_start)))
                }
            };
            match self.list_iter.next() {
                Some(&DiffListElem::Array(ref array)) => {
                    self.info_iter = Some(array.iter());
                },
                Some(&DiffListElem::Inst(ref inst)) => {
                    self.info_iter = None;
                    return Some(ImmCow::new_borrowed(inst))
                },
                None => {
                    self.info_iter = None;
                    return None
                },
            };
        }
    }
}

#[derive(Clone)]
pub struct ResourceTableInfoIter<'a, 'list>
    where 'a: 'list
{
    iter: ResourceTableIterator<'a, 'list>,
    offset: usize
}

impl<'a, 'list> Iterator for ResourceTableInfoIter<'a, 'list>
    where 'a: 'list
{
    type Item = ResourceInfo;
    fn next(&mut self) -> Option<Self::Item>
    {
        if let Some(res) = self.iter.next() {
            let info = res.resource_info(self.offset as u32);
            self.offset += res.size();
            Some(info)
        } else {
            None
        }
    }
}

#[derive(Debug)]
enum DiffListElem<'a>
{
    Array(InfoArray<'a>),
    Inst(Resource_<'a>), // XXX Box?
}

impl<'a> DiffListElem<'a>
{
    fn len(&self) -> usize
    {
        match *self {
            DiffListElem::Array(ref array) => array.len(),
            DiffListElem::Inst(_) => 1,
        }
    }
}

//#[derive(Debug, Clone)]
#[derive(Debug)]
pub struct Resource_<'a>
{
    pub compressed: bool,
    pub fourcc: FourCC,
    pub file_id: u32,
    pub kind: ResourceKind<'a>,
    #[cfg(debug_assertions)]
    pub original_offset: u32,
}

impl<'a> Resource_<'a>
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
        }
    }
}

impl<'a> Readable<'a> for Resource_<'a>
{
    type Args = ResourceInfo;
    #[cfg(debug_assertions)]
    fn read(reader: Reader<'a>, info: Self::Args) -> (Self, Reader<'a>)
    {
        if info.compressed > 1 {
            panic!("Bad info.compressed")
        };
        let res = Resource_ {
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
        let res = Resource_ {
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
        }
    }
}

impl<'a> Writable for Resource_<'a>
{
    fn write<W: Write>(&self, writer: &mut W)
    {
        match self.kind {
            ResourceKind::Unknown(ref reader) => writer.write_all(&reader).unwrap(),
            ResourceKind::Mrea(ref mrea) => mrea.write(writer),
        }
    }
}

#[derive(Debug)]
pub enum ResourceKind<'a>
{
    Unknown(Reader<'a>),
    Mrea(Mrea<'a>),
    //UnknownCompressed(Reader<'a>),
}

