use std::{
    fmt,
    slice::Iter as SliceIter,
};

use crate::{
    lcow::LCow,
    reader::{Reader, Readable, ReaderEx},
    writer::{Writable, Writer},
};


/// A lazy array with one element for each element in an iterator (often another array).
#[derive(Clone)]
pub enum IteratorArray<R, T, I>
    where R: Reader,
          T: Readable<R>,
          I: Iterator<Item=Result<T::Args, R::Error>> + ExactSizeIterator + Clone
{
    Borrowed(R, I),
    Owned(Vec<T>),
}

impl<R, T, I> IteratorArray<R, T, I>
    where R: Reader,
          T: Readable<R>,
          I: Iterator<Item=Result<T::Args, R::Error>> + ExactSizeIterator + Clone
{
    pub fn len(&self) -> usize
    {
        match *self {
            IteratorArray::Borrowed(_, ref i) => i.len(),
            IteratorArray::Owned(ref vec) => vec.len(),
        }
    }

    pub fn iter<'s>(&'s self) -> IteratorArrayIterator<'s, R, T, I>
    {
        match *self {
            IteratorArray::Borrowed(ref reader, ref i)
                => IteratorArrayIterator::Borrowed(reader.clone(), i.clone()),
            IteratorArray::Owned(ref vec) => IteratorArrayIterator::Owned(vec.iter()),
        }
    }

    pub fn as_mut_vec(&mut self) -> Result<&mut Vec<T>, R::Error>
    {
        *self = match self {
            IteratorArray::Borrowed(reader, iter) => {
                let res: Result<_, _> = iter
                    .map(|a| a.and_then(|arg| reader.read(arg)))
                    .collect();
                IteratorArray::Owned(res?)
            },
            IteratorArray::Owned(vec) => return Ok(vec),
        };
        match self {
            IteratorArray::Owned(vec) => Ok(vec),
            IteratorArray::Borrowed(_, _) => unreachable!(),
        }
    }
}

impl<R, T, I> Readable<R> for IteratorArray<R, T, I>
    where R: Reader,
          T: Readable<R>,
          I: Iterator<Item=Result<T::Args, R::Error>> + ExactSizeIterator + Clone
{
    type Args = I;
    fn read_from(reader: &mut R, i: I) -> Result<Self, R::Error>
    {
        let res = IteratorArray::Borrowed(reader.clone(), i);
        reader.advance(res.size()?)?;
        Ok(res)
    }

    fn size(&self) -> Result<usize, R::Error>
    {
        if let Some(i) = T::fixed_size() {
            Ok(i * self.len())
        } else {
            self.iter().try_fold(0, |s, i| Ok(s + i?.size()?))
        }
    }
}

#[derive(Clone)]
pub enum IteratorArrayIterator<'s, R: 's, T, I>
    where R: Reader,
          T: Readable<R> + 's,
          I: Iterator<Item=Result<T::Args, R::Error>> + ExactSizeIterator + Clone
{
    Borrowed(R, I),
    Owned(SliceIter<'s, T>),
}

impl<'s, R: 's, T, I> Iterator for IteratorArrayIterator<'s, R, T, I>
    where R: Reader,
          T: Readable<R> + 's,
          I: Iterator<Item=Result<T::Args, R::Error>> + ExactSizeIterator + Clone
{
    type Item = Result<LCow<'s, T>, R::Error>;
    fn next(&mut self) -> Option<Self::Item>
    {
        match *self {
            IteratorArrayIterator::Borrowed(ref mut reader, ref mut args_iter) => {
                if let Some(args) = args_iter.next() {
                    // XXX Ideal place for a try-block. Sigh...
                    let res = args.and_then(|args| reader.read::<T>(args));
                    Some(res.map(LCow::Owned))
                } else {
                    None
                }
            },
            IteratorArrayIterator::Owned(ref mut iter) => iter.next().map(|i| Ok(LCow::Borrowed(i))),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>)
    {
        match *self {
            IteratorArrayIterator::Borrowed(_, ref args_iter) => args_iter.size_hint(),
            IteratorArrayIterator::Owned(ref iter) => iter.size_hint(),
        }
    }
}

impl<'s, R: 's, T, I> ExactSizeIterator for IteratorArrayIterator<'s, R, T, I>
    where R: Reader,
          T: Readable<R> + 's,
          I: Iterator<Item=Result<T::Args, R::Error>> + ExactSizeIterator + Clone
{
    fn len(&self) -> usize
    {
        match *self {
            IteratorArrayIterator::Borrowed(_, ref args_iter) => args_iter.len(),
            IteratorArrayIterator::Owned(ref iter) => iter.len(),
        }
    }
}

impl<R, W, T, I> Writable<W> for IteratorArray<R, T, I>
    where R: Reader,
          W: Writer,
          T: Readable<R> + Writable<W>,
          I: Iterator<Item=Result<T::Args, R::Error>> + ExactSizeIterator + Clone,
          W::Error: From<R::Error>,
{
    fn write_to(&self, writer: &mut W) -> Result<u64, W::Error>
    {
        let mut s = 0;
        for i in self.iter() {
            s += i?.write_to(writer)?
        }
        Ok(s)
    }
}

impl<R, T, I> fmt::Debug for IteratorArray<R, T, I>
    where R: Reader,
          T: Readable<R> + fmt::Debug,
          I: Iterator<Item=Result<T::Args, R::Error>> + ExactSizeIterator + Clone
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        match self {
            IteratorArray::Borrowed(_, _) => {
                let res: Result<Vec<_>, _> = self.iter().collect();
                fmt::Debug::fmt(
                    &res.unwrap_or_else(|_| panic!("Error while fmting a IteratorArray")),
                    f
                )
            }
            IteratorArray::Owned(vec) => fmt::Debug::fmt(vec, f),
        }
    }
}

impl<R, T, I> From<Vec<T>> for IteratorArray<R, T, I>
    where R: Reader,
          T: Readable<R>,
          I: Iterator<Item=Result<T::Args, R::Error>> + ExactSizeIterator + Clone
{
    fn from(vec: Vec<T>) -> IteratorArray<R, T, I>
    {
        IteratorArray::Owned(vec)
    }
}

