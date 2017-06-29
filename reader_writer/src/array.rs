use std::fmt;
use std::io;
use std::slice::Iter as SliceIter;
use std::slice::IterMut as SliceIterMut;

use lcow::LCow;
use reader::{Reader, Readable};
use writer::Writable;

use read_only_array::{RoArray, RoArrayIter};

impl<'a, T> Readable<'a> for Vec<T>
    where T: Readable<'a>,
          T::Args: Clone,
{
    type Args = (usize, T::Args);
    #[inline]
    fn read(mut reader: Reader<'a>, (len, args): Self::Args) -> (Self, Reader<'a>)
    {
        let mut res = Vec::with_capacity(len);
        for _ in 0..len {
            res.push(reader.read(args.clone()));
        };
        (res, reader)
    }

    #[inline]
    fn size(&self) -> usize
    {
        T::fixed_size()
            .map(|i| i * self.len())
            .unwrap_or_else(|| self.iter().fold(0, |s, i| s + i.size()))
    }
}

impl<'a, T> Writable for Vec<T>
    where T: Writable,
{
    #[inline]
    fn write<W: io::Write>(&self, writer: &mut W) -> io::Result<()>
    {
        for i in self {
            i.write(writer)?
        }
        Ok(())
    }
}

#[derive(Clone)]
pub enum LazyArray<'a, T>
    where T: Readable<'a>,
          T::Args: Clone,
{
    Borrowed(RoArray<'a, T>),
    Owned(Vec<T>),
}


impl<'a, T> LazyArray<'a, T>
    where T: Readable<'a>,
          T::Args: Clone,
{
    #[inline]
    pub fn len(&self) -> usize
    {
        match *self {
            LazyArray::Borrowed(ref array) => array.len(),
            LazyArray::Owned(ref vec) => vec.len(),
        }
    }

    #[inline]
    pub fn iter<'s>(&'s self) -> LazyArrayIter<'s, 'a, T>
    {
        self.into_iter()
    }

    #[inline]
    pub fn iter_mut<'s>(&'s mut self) -> SliceIterMut<'s, T>
    {
        self.as_mut_vec().iter_mut()
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<LCow<T>>
    {
        match *self {
            LazyArray::Borrowed(ref array) => array.get(index).map(LCow::Owned),
            LazyArray::Owned(ref vec) => vec.get(index).map(LCow::Borrowed),
        }
    }

    pub fn split_off(&mut self, at: usize) -> LazyArray<'a, T>
    {
        match *self {
            LazyArray::Borrowed(ref mut array) => LazyArray::Borrowed(array.split_off(at)),
            LazyArray::Owned(ref mut vec) => LazyArray::Owned(vec.split_off(at)),
        }
    }

    pub fn as_mut_vec(&mut self) -> &mut Vec<T>
    {
        *self = match *self {
            LazyArray::Borrowed(ref array) => LazyArray::Owned(array.iter().collect()),
            LazyArray::Owned(ref mut vec) => return vec,
        };
        match *self {
            LazyArray::Owned(ref mut vec) => vec,
            LazyArray::Borrowed(_) => unreachable!(),
        }
    }

    pub fn is_owned(&self) -> bool
    {
        match *self {
            LazyArray::Borrowed(_) => false,
            LazyArray::Owned(_) => true,
        }
    }
}

impl<'a, T> Readable<'a> for LazyArray<'a, T>
    where T: Readable<'a>,
          T::Args: Clone,
{
    type Args = (usize, T::Args);

    #[inline]
    fn read(reader: Reader<'a>, args: Self::Args) -> (Self, Reader<'a>)
    {
        let (array, reader) = RoArray::read(reader, args);
        (LazyArray::Borrowed(array), reader)
    }

    #[inline]
    fn size(&self) -> usize
    {
        T::fixed_size()
            .map(|i| i * self.len())
            .unwrap_or_else(|| self.iter().fold(0, |s, i| s + i.size()))
    }
}

impl<'a, T> fmt::Debug for LazyArray<'a, T>
    where T: Readable<'a> + fmt::Debug,
          T::Args: Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        fmt::Debug::fmt(&self.iter().collect::<Vec<_>>(), f)
    }
}


#[derive(Clone)]
pub enum LazyArrayIter<'s, 'a, T>
    where T: Readable<'a> + 's,
          T::Args: Clone,
{
    Borrowed(RoArrayIter<'a, T>),
    Owned(SliceIter<'s, T>),
}

impl<'s, 'a, T> Iterator for LazyArrayIter<'s, 'a, T>
    where T: Readable<'a>,
          T::Args: Clone,
{
    type Item = LCow<'s, T>;
    #[inline]
    fn next(&mut self) -> Option<Self::Item>
    {
        match *self {
            LazyArrayIter::Borrowed(ref mut iter) => iter.next().map(LCow::Owned),
            LazyArrayIter::Owned(ref mut iter) => iter.next().map(LCow::Borrowed),
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>)
    {
        match *self {
            LazyArrayIter::Borrowed(ref iter) => iter.size_hint(),
            LazyArrayIter::Owned(ref iter) => iter.size_hint()
        }
    }
}

impl<'s, 'a, T> ExactSizeIterator for LazyArrayIter<'s, 'a, T>
    where T: Readable<'a>,
          T::Args: Clone,
{
    #[inline]
    fn len(&self) -> usize
    {
        match *self {
            LazyArrayIter::Borrowed(ref iter) => iter.len(),
            LazyArrayIter::Owned(ref iter) => iter.len()
        }
    }
}

impl<'s, 'a, T: 's> IntoIterator for &'s LazyArray<'a, T>
    where T: Readable<'a>,
          T::Args: Clone,
{
    type Item = LCow<'s, T>;
    type IntoIter = LazyArrayIter<'s, 'a, T>;
    #[inline]
    fn into_iter(self) -> Self::IntoIter
    {
        match *self {
            LazyArray::Borrowed(ref array) => LazyArrayIter::Borrowed(array.iter()),
            LazyArray::Owned(ref vec) => LazyArrayIter::Owned(vec.iter()),
        }
    }
}


impl<'a, T> Writable for LazyArray<'a, T>
    where T: Readable<'a> + Writable,
          T::Args: Clone,
{
    #[inline]
    fn write<W: io::Write>(&self, writer: &mut W) -> io::Result<()>
    {
        match *self {
            LazyArray::Borrowed(ref array) => array.write(writer),
            LazyArray::Owned(ref vec) => <Vec<T> as Writable>::write(&vec, writer),
        }
    }
}

impl<'a, T> From<Vec<T>> for LazyArray<'a, T>
    where T: Readable<'a>,
          T::Args: Clone,
{
    fn from(vec: Vec<T>) -> LazyArray<'a, T>
    {
        LazyArray::Owned(vec)
    }
}
