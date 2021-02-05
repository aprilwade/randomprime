use std::fmt;
use std::slice::Iter as SliceIter;
use std::slice::IterMut as SliceIterMut;

use crate::lcow::LCow;
use crate::reader::{Reader, Readable, ReaderEx};
use crate::writer::{Writable, Writer};
use crate::read_only_array::{RoArray, RoArrayIter};
use crate::derivable_array_proxy::DerivableFromIterator;

impl<R, T> Readable<R> for Vec<T>
    where R: Reader,
          T: Readable<R>,
          T::Args: Clone,
{
    type Args = (usize, T::Args);
    fn read_from(reader: &mut R, (len, args): Self::Args) -> Result<Self, R::Error>
    {
        (0..len).into_iter().map(|_| reader.read(args.clone())).collect()
    }

    fn size(&self) -> Result<usize, R::Error>
    {
        T::fixed_size()
            .map(|i| Ok(i * self.len()))
            .unwrap_or_else(|| self.iter().try_fold(0, |s, i| Ok(s + i.size()?)))
    }
}

impl<W: Writer, T> Writable<W>for Vec<T>
    where T: Writable<W>,
{
    fn write_to(&self, writer: &mut W) -> Result<u64, W::Error>
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

pub enum LazyArray<R, T>
    where R: Reader,
          T: Readable<R>,
{
    Borrowed(RoArray<R, T>),
    Owned(Vec<T>),
}

impl<R, T> Clone for LazyArray<R, T>
    where R: Reader,
          T: Readable<R> + Clone,
          T::Args: Clone,
{
    fn clone(&self) -> Self
    {
        match self {
            LazyArray::Borrowed(array) => LazyArray::Borrowed(array.clone()),
            LazyArray::Owned(vec) => LazyArray::Owned(vec.clone()),
        }
    }
}

impl<R, T> LazyArray<R, T>
    where R: Reader,
          T: Readable<R>,
{
    pub fn len(&self) -> usize
    {
        match *self {
            LazyArray::Borrowed(ref array) => array.len(),
            LazyArray::Owned(ref vec) => vec.len(),
        }
    }

    pub fn iter<'s>(&'s self) -> LazyArrayIter<'s, R, T>
          where T::Args: Clone,
    {
        self.into_iter()
    }

    pub fn iter_mut<'s>(&'s mut self) -> Result<SliceIterMut<'s, T>, R::Error>
          where T::Args: Clone,
    {
        Ok(self.as_mut_vec()?.iter_mut())
    }

    pub fn get(&self, index: usize) -> Result<Option<LCow<T>>, R::Error>
          where T::Args: Clone,
    {
        match *self {
            LazyArray::Borrowed(ref array) => Ok(array.get(index)?.map(LCow::Owned)),
            LazyArray::Owned(ref vec) => Ok(vec.get(index).map(LCow::Borrowed)),
        }
    }

    pub fn split_off(&mut self, at: usize) -> Result<LazyArray<R, T>, R::Error>
          where T::Args: Clone,
    {
        match *self {
            LazyArray::Borrowed(ref mut array) => Ok(LazyArray::Borrowed(array.split_off(at)?)),
            LazyArray::Owned(ref mut vec) => Ok(LazyArray::Owned(vec.split_off(at))),
        }
    }

    pub fn as_mut_vec(&mut self) -> Result<&mut Vec<T>, R::Error>
          where T::Args: Clone,
    {
        *self = match self {
            LazyArray::Borrowed(array) => {
                LazyArray::Owned(array.iter().collect::<Result<_, _>>()?)
            },
            LazyArray::Owned(vec) => return Ok(vec),
        };
        match self {
            LazyArray::Owned(vec) => Ok(vec),
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

impl<R, T> Readable<R> for LazyArray<R, T>
    where R: Reader,
          T: Readable<R>,
          T::Args: Clone,
{
    type Args = (usize, T::Args);

    fn read_from(reader: &mut R, args: Self::Args) -> Result<Self, R::Error>
    {
        let array = RoArray::read_from(reader, args)?;
        Ok(LazyArray::Borrowed(array))
    }

    fn size(&self) -> Result<usize, R::Error>
    {
        T::fixed_size()
            .map(|i| Ok(i * self.len()))
            .unwrap_or_else(|| self.iter().try_fold(0, |s, i| Ok(s + i?.size()?)))
    }
}

impl<R, T> DerivableFromIterator for LazyArray<R, T>
    where R: Reader,
          T: Readable<R>,
          T::Args: Clone,
{
    type Item = T;
}

impl<R, T> fmt::Debug for LazyArray<R, T>
    where R: Reader,
          T: Readable<R> + fmt::Debug,
          T::Args: Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        match self {
            LazyArray::Borrowed(array) => fmt::Debug::fmt(array, f),
            LazyArray::Owned(vec) => fmt::Debug::fmt(vec, f),
        }
    }
}


#[derive(Clone)]
pub enum LazyArrayIter<'s, R, T>
    where R: Reader,
          T: Readable<R> + 's,
          T::Args: Clone,
{
    Borrowed(RoArrayIter<R, T>),
    Owned(SliceIter<'s, T>),
}

impl<'s, R, T> Iterator for LazyArrayIter<'s, R, T>
    where R: Reader,
          T: Readable<R>,
          T::Args: Clone,
{
    type Item = Result<LCow<'s, T>, R::Error>;
    fn next(&mut self) -> Option<Self::Item>
    {
        match *self {
            LazyArrayIter::Borrowed(ref mut iter) => iter.next().map(|i| i.map(LCow::Owned)),
            LazyArrayIter::Owned(ref mut iter) => iter.next().map(|i| Ok(LCow::Borrowed(i))),
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

impl<'s, R, T> ExactSizeIterator for LazyArrayIter<'s, R, T>
    where R: Reader,
          T: Readable<R>,
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

impl<'s, R, T: 's> IntoIterator for &'s LazyArray<R, T>
    where R: Reader,
          T: Readable<R>,
          T::Args: Clone,
{
    type Item = Result<LCow<'s, T>, R::Error>;
    type IntoIter = LazyArrayIter<'s, R, T>;
    fn into_iter(self) -> Self::IntoIter
    {
        match *self {
            LazyArray::Borrowed(ref array) => LazyArrayIter::Borrowed(array.iter()),
            LazyArray::Owned(ref vec) => LazyArrayIter::Owned(vec.iter()),
        }
    }
}


impl<R, W, T> Writable<W>for LazyArray<R, T>
    where R: Reader,
          W: Writer,
          T: Readable<R> + Writable<W>,
          T::Args: Clone,
          W::Error: From<R::Error>
{
    fn write_to(&self, writer: &mut W) -> Result<u64, W::Error>
    {
        match *self {
            LazyArray::Borrowed(ref array) => array.write_to(writer),
            LazyArray::Owned(ref vec) => <Vec<T> as Writable<W>>::write_to(&vec, writer),
        }
    }
}

impl<R, T> From<Vec<T>> for LazyArray<R, T>
    where R: Reader,
          T: Readable<R>,
          T::Args: Clone,
{
    fn from(vec: Vec<T>) -> LazyArray<R, T>
    {
        LazyArray::Owned(vec)
    }
}
