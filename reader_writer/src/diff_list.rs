
use std::fmt;
use std::io::Write;
use std::iter::{once, FromIterator};

use linked_list::{Cursor as LinkedListCursor, Iter as LinkedListIter, LinkedList};

use reader::{Reader, Readable};
use writer::Writable;
use imm_cow::ImmCow;

pub trait DiffListSourceCursor
{
    type Item;
    type Source;

    /// `true` if the cursor was successfully advanced, `false` if not.
    fn next(&mut self) -> bool;
    fn get(&self) -> Self::Item;
    /// Returns the source of the cursor split in two. The current element goes into the
    /// right return value.
    fn split(self) -> (Option<Self::Source>, Self::Source);
    fn split_around(self) -> (Option<Self::Source>, Self::Item, Option<Self::Source>);
}

pub trait AsDiffListSourceCursor: Sized
{
    type Cursor: DiffListSourceCursor<Source=Self>;
    fn as_cursor(&self) -> Self::Cursor;
    fn len(&self) -> usize;
}


pub struct DiffList<'a, A>
    where A: AsDiffListSourceCursor,
{
    data_start: Reader<'a>,
    list: LinkedList<DiffListElem<A>>,
}

impl<'a, A> Clone for DiffList<'a, A>
    where A: AsDiffListSourceCursor + Clone,
          <A::Cursor as DiffListSourceCursor>::Item: Clone,
{
    fn clone(&self) -> Self
    {
        DiffList {
            data_start: self.data_start.clone(),
            list: self.list.clone(),
        }
    }
}

impl<'a, A> fmt::Debug for DiffList<'a, A>
    where A: AsDiffListSourceCursor + fmt::Debug,
          <A::Cursor as DiffListSourceCursor>::Item: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error>
    {
        write!(f, "DiffList {{ data_start: {:?}, list: {:?} }}", self.data_start, self.list)
    }
}

pub enum DiffListElem<A>
    where A: AsDiffListSourceCursor,
{
    Array(A),
    Inst(<A::Cursor as DiffListSourceCursor>::Item),
}

impl<A> Clone for DiffListElem<A>
    where A: AsDiffListSourceCursor + Clone,
          <A::Cursor as DiffListSourceCursor>::Item: Clone,
{
    fn clone(&self) -> Self
    {
        match *self {
            DiffListElem::Array(ref a) => DiffListElem::Array(a.clone()),
            DiffListElem::Inst(ref i) => DiffListElem::Inst(i.clone()),
        }
    }
}


impl<A> fmt::Debug for DiffListElem<A>
    where A: AsDiffListSourceCursor + fmt::Debug,
          <A::Cursor as DiffListSourceCursor>::Item: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error>
    {
        match *self {
            DiffListElem::Array(ref a) => write!(f, "DiffListElem::Array({:?})", *a),
            DiffListElem::Inst(ref i) => write!(f, "DiffListElem::Inst({:?})", i),
        }
    }
}

impl<'a, A> DiffList<'a, A>
    where A: AsDiffListSourceCursor,
{
    pub fn cursor<'s>(&'s mut self) -> DiffListCursor<'s, A>
    {
        let mut cursor = self.list.cursor();
        DiffListCursor {
            inner_cursor: match cursor.peek_next() {
                Some(&mut DiffListElem::Array(ref a)) => Some(a.as_cursor()),
                _ => None,
            },
            cursor: cursor,
        }
    }

    pub fn iter<'s>(&'s self) -> DiffListIter<'s, A>
    {
        DiffListIter {
            list_iter: self.list.iter(),
            inner_cursor: None,
        }
    }

    pub fn elems_iter<'s>(&'s self) -> LinkedListIter<DiffListElem<A>>
    {
        self.list.iter()
    }

    pub fn len(&self) -> usize
    {
        // TODO: It might make sense to cache this...
        self.list.iter().map(|elem| elem.len()).sum()
    }
}

impl<A> DiffListElem<A>
    where A: AsDiffListSourceCursor,
{
    fn len(&self) -> usize
    {
        match *self {
            DiffListElem::Array(ref array) => array.len(),
            DiffListElem::Inst(_) => 1,
        }
    }
}


pub struct DiffListCursor<'list, A>
    where A: AsDiffListSourceCursor + 'list,
{
    cursor: LinkedListCursor<'list, DiffListElem<A>>,
    inner_cursor: Option<A::Cursor>,
}

impl<'list, A> DiffListCursor<'list, A>
    where A: AsDiffListSourceCursor + 'list,
{
    // TODO: Return value?
    pub fn next(&mut self)
    {
        let advance_cursor = self.inner_cursor.as_mut().map(|ic| !ic.next()).unwrap_or(true);
        if advance_cursor {
            self.inner_cursor = None;
            self.cursor.next();
            match self.cursor.peek_next() {
                None => (),
                Some(&mut DiffListElem::Inst(_)) => (),
                Some(&mut DiffListElem::Array(ref a)) => {
                    self.inner_cursor = Some(a.as_cursor());
                },
            };
        };
    }

    // TODO: prev?

    /// Inserts the items yielded by `iter` into the list. The cursor will be
    /// positioned at the first inserted item.
    pub fn insert_before<I>(&mut self, iter: I)
        where I: Iterator<Item=<A::Cursor as DiffListSourceCursor>::Item>
    {
        let mut list = LinkedList::from_iter(iter.map(DiffListElem::Inst));
        if list.len() == 0 {
            return;
        };

        if let Some(ic) = self.inner_cursor.take() {
            let (left, right) = ic.split();
            if let Some(left) = left {
                self.cursor.insert(DiffListElem::Array(left));
                self.cursor.next();
            };
            *self.cursor.peek_next().unwrap() = DiffListElem::Array(right);
        };
        self.cursor.splice(&mut list);
        // We shouldn't need to set self.inner_cursor here. We've inserted at
        // least one element, so self.cursor should be pointing to an Inst.
    }

    pub fn peek(&mut self) -> Option<ImmCow<<A::Cursor as DiffListSourceCursor>::Item>>
    {
        if let Some(ref ic) = self.inner_cursor {
            Some(ImmCow::new_owned(ic.get()))
        } else {
            match self.cursor.peek_next() {
                None => None,
                Some(&mut DiffListElem::Array(_)) => unreachable!(),
                Some(&mut DiffListElem::Inst(ref res)) => Some(ImmCow::new_borrowed(res)),
            }
        }
    }

    pub fn value(&mut self) -> Option<&mut <A::Cursor as DiffListSourceCursor>::Item>
    {
        if let Some(ic) = self.inner_cursor.take() {
            let (left, elem, right) = ic.split_around();
            if let Some(right) = right {
                // There are elements to the right
                *self.cursor.peek_next().unwrap() = DiffListElem::Array(right);
                self.cursor.insert(DiffListElem::Inst(elem));
            } else {
                // This was the last element.
                *self.cursor.peek_next().unwrap() = DiffListElem::Inst(elem);
            };
            // self.cursor now points to the correct Inst
            if let Some(left) = left {
                // There are elements to the left.
                self.cursor.insert(DiffListElem::Array(left));
                self.cursor.next();
            };
        };
        match self.cursor.peek_next() {
            Some(&mut DiffListElem::Inst(ref mut inst)) => Some(inst),
            Some(&mut DiffListElem::Array(_)) => unreachable!(),
            None => None,
        }
    }

}

#[derive(Clone)]
pub struct DiffListIter<'list, A>
    where A: AsDiffListSourceCursor + 'list,
{
    list_iter: LinkedListIter<'list, DiffListElem<A>>,
    inner_cursor: Option<A::Cursor>,
}

impl<'list, A> Iterator for DiffListIter<'list, A>
    where A: AsDiffListSourceCursor + 'list,
{
    type Item = ImmCow<'list, <A::Cursor as DiffListSourceCursor>::Item>;
    fn next(&mut self) -> Option<Self::Item>
    {
        if let Some(ref mut cursor) = self.inner_cursor {
            if cursor.next() {
                return Some(ImmCow::new_owned(cursor.get()))
            }
        }
        match self.list_iter.next() {
            Some(&DiffListElem::Array(ref array)) => {
                let cursor = array.as_cursor();
                let res = cursor.get();
                self.inner_cursor = Some(cursor);
                Some(ImmCow::new_owned(res))
            },
            Some(&DiffListElem::Inst(ref inst)) => Some(ImmCow::new_borrowed(inst)),
            None => None,
        }
    }
}

impl<'a, A> Readable<'a> for DiffList<'a, A>
    where A: AsDiffListSourceCursor,
          <A::Cursor as DiffListSourceCursor>::Item: Readable<'a>,
{
    type Args = A;
    fn read(reader: Reader<'a>, args: A) -> (Self, Reader<'a>)
    {
        let res = DiffList {
            list: LinkedList::from_iter(once(DiffListElem::Array(args))),
            data_start: reader.clone(),
        };
        let size = res.size();
        (res, reader.offset(size))
    }

    fn size(&self) -> usize
    {
        <A::Cursor as DiffListSourceCursor>::Item::fixed_size()
            .map(|i| i * self.len())
            .unwrap_or_else(|| self.iter().fold(0, |s, i| s + i.size()))
    }
}

impl<'a, A> Writable for DiffList<'a, A>
    where A: AsDiffListSourceCursor,
          <A::Cursor as DiffListSourceCursor>::Item: Writable,
{
    fn write<W: Write>(&self, writer: &mut W)
    {
        for i in self.iter() {
            i.write(writer);
        }
    }
}
