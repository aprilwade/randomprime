use std::fmt;
use std::io;
use std::slice::Iter as SliceIter;
use std::slice::IterMut as SliceIterMut;

use crate::lcow::LCow;
use crate::reader::{Reader, Readable};
use crate::writer::Writable;
use crate::read_only_array::{RoArray, RoArrayIter};
use crate::derivable_array_proxy::DerivableFromIterator;

impl<'r, T> Readable<'r> for Vec<T>
    where T: Readable<'r>,
          T::Args: Clone,
{
    type Args = (usize, T::Args);
    fn read_from(reader: &mut Reader<'r>, (len, args): Self::Args) -> Self
    {
        let mut res = Vec::with_capacity(len);
        for _ in 0..len {
            res.push(reader.read(args.clone()));
        };
        res
    }

    fn size(&self) -> usize
    {
        T::fixed_size()
            .map(|i| i * self.len())
            .unwrap_or_else(|| self.iter().fold(0, |s, i| s + i.size()))
    }
}

impl<'r, T> Writable for Vec<T>
    where T: Writable,
{
    fn write_to<W: io::Write>(&self, writer: &mut W) -> io::Result<u64>
    {
        let mut s = 0;
        for i in self {
            s += i.write_to(writer)?
        }
        Ok(s)
    }
}

impl<T> DerivableFromIterator for Vec<T>
{
    type Item = T;
}

#[derive(Clone)]
pub enum LazyArray<'r, T>
    where T: Readable<'r>,
          T::Args: Clone,
{
    Borrowed(RoArray<'r, T>),
    Owned(Vec<T>),
}


impl<'r, T> LazyArray<'r, T>
    where T: Readable<'r>,
          T::Args: Clone,
{
    pub fn len(&self) -> usize
    {
        match *self {
            LazyArray::Borrowed(ref array) => array.len(),
            LazyArray::Owned(ref vec) => vec.len(),
        }
    }

    pub fn iter<'s>(&'s self) -> LazyArrayIter<'s, 'r, T>
    {
        self.into_iter()
    }

    pub fn iter_mut<'s>(&'s mut self) -> SliceIterMut<'s, T>
    {
        self.as_mut_vec().iter_mut()
    }

    pub fn get(&self, index: usize) -> Option<LCow<T>>
    {
        match *self {
            LazyArray::Borrowed(ref array) => array.get(index).map(LCow::Owned),
            LazyArray::Owned(ref vec) => vec.get(index).map(LCow::Borrowed),
        }
    }

    pub fn split_off(&mut self, at: usize) -> LazyArray<'r, T>
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

impl<'r, T> Readable<'r> for LazyArray<'r, T>
    where T: Readable<'r>,
          T::Args: Clone,
{
    type Args = (usize, T::Args);

    fn read_from(reader: &mut Reader<'r>, args: Self::Args) -> Self
    {
        let array = RoArray::read_from(reader, args);
        LazyArray::Borrowed(array)
    }

    fn size(&self) -> usize
    {
        T::fixed_size()
            .map(|i| i * self.len())
            .unwrap_or_else(|| self.iter().fold(0, |s, i| s + i.size()))
    }
}

impl<'r, T> DerivableFromIterator for LazyArray<'r, T>
    where T: Readable<'r>,
          T::Args: Clone,
{
    type Item = T;
}

impl<'r, T> fmt::Debug for LazyArray<'r, T>
    where T: Readable<'r> + fmt::Debug,
          T::Args: Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        fmt::Debug::fmt(&self.iter().collect::<Vec<_>>(), f)
    }
}


#[derive(Clone)]
pub enum LazyArrayIter<'s, 'r, T>
    where T: Readable<'r> + 's,
          T::Args: Clone,
{
    Borrowed(RoArrayIter<'r, T>),
    Owned(SliceIter<'s, T>),
}

impl<'s, 'r, T> Iterator for LazyArrayIter<'s, 'r, T>
    where T: Readable<'r>,
          T::Args: Clone,
{
    type Item = LCow<'s, T>;
    fn next(&mut self) -> Option<Self::Item>
    {
        match *self {
            LazyArrayIter::Borrowed(ref mut iter) => iter.next().map(LCow::Owned),
            LazyArrayIter::Owned(ref mut iter) => iter.next().map(LCow::Borrowed),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>)
    {
        match *self {
            LazyArrayIter::Borrowed(ref iter) => iter.size_hint(),
            LazyArrayIter::Owned(ref iter) => iter.size_hint()
        }
    }
}

impl<'s, 'r, T> ExactSizeIterator for LazyArrayIter<'s, 'r, T>
    where T: Readable<'r>,
          T::Args: Clone,
{
    fn len(&self) -> usize
    {
        match *self {
            LazyArrayIter::Borrowed(ref iter) => iter.len(),
            LazyArrayIter::Owned(ref iter) => iter.len()
        }
    }
}

impl<'s, 'r, T: 's> IntoIterator for &'s LazyArray<'r, T>
    where T: Readable<'r>,
          T::Args: Clone,
{
    type Item = LCow<'s, T>;
    type IntoIter = LazyArrayIter<'s, 'r, T>;
    fn into_iter(self) -> Self::IntoIter
    {
        match *self {
            LazyArray::Borrowed(ref array) => LazyArrayIter::Borrowed(array.iter()),
            LazyArray::Owned(ref vec) => LazyArrayIter::Owned(vec.iter()),
        }
    }
}


impl<'r, T> Writable for LazyArray<'r, T>
    where T: Readable<'r> + Writable,
          T::Args: Clone,
{
    fn write_to<W: io::Write>(&self, writer: &mut W) -> io::Result<u64>
    {
        match *self {
            LazyArray::Borrowed(ref array) => array.write_to(writer),
            LazyArray::Owned(ref vec) => <Vec<T> as Writable>::write_to(&vec, writer),
        }
    }
}

impl<'r, T> From<Vec<T>> for LazyArray<'r, T>
    where T: Readable<'r>,
          T::Args: Clone,
{
    fn from(vec: Vec<T>) -> LazyArray<'r, T>
    {
        LazyArray::Owned(vec)
    }
}
