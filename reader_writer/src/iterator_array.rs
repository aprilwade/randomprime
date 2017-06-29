use std::fmt;
use std::io;
use std::slice::Iter as SliceIter;

use lcow::LCow;
use reader::{Reader, Readable};
use writer::Writable;


#[derive(Clone)]
pub enum IteratorArray<'a, T, I>
    where T: Readable<'a>,
          I: Iterator<Item=T::Args> + ExactSizeIterator + Clone
{
    Borrowed(Reader<'a>, I),
    Owned(Vec<T>),
}

impl<'a, T, I> IteratorArray<'a, T, I>
    where T: Readable<'a>,
          I: Iterator<Item=T::Args> + ExactSizeIterator + Clone
{
    #[inline]
    pub fn len(&self) -> usize
    {
        match *self {
            IteratorArray::Borrowed(_, ref i) => i.len(),
            IteratorArray::Owned(ref vec) => vec.len(),
        }
    }

    #[inline]
    pub fn iter<'s>(&'s self) -> IteratorArrayIterator<'s, 'a, T, I>
    {
        match *self {
            IteratorArray::Borrowed(ref reader, ref i)
                => IteratorArrayIterator::Borrowed(reader.clone(), i.clone()),
            IteratorArray::Owned(ref vec) => IteratorArrayIterator::Owned(vec.iter()),
        }
    }

    pub fn as_mut_vec(&mut self) -> &mut Vec<T>
    {
        *self = match *self {
            IteratorArray::Borrowed(ref mut reader, ref mut iter) => {
                let mut vec = Vec::with_capacity(iter.len());
                while let Some(arg) = iter.next() {
                    vec.push(reader.read(arg));
                };
                IteratorArray::Owned(vec)
            },
            IteratorArray::Owned(ref mut vec) => return vec,
        };
        match *self {
            IteratorArray::Owned(ref mut vec) => vec,
            IteratorArray::Borrowed(_, _) => unreachable!(),
        }
    }
}

impl<'a, T, I> Readable<'a> for IteratorArray<'a, T, I>
    where T: Readable<'a>,
          I: Iterator<Item=T::Args> + ExactSizeIterator + Clone
{
    type Args = I;
    fn read(mut reader: Reader<'a>, i: I) -> (Self, Reader<'a>)
    {
        let res = IteratorArray::Borrowed(reader.clone(), i);
        reader.advance(res.size());
        (res, reader)
    }

    fn size(&self) -> usize
    {
        if let Some(i) = T::fixed_size() {
            i * self.len()
        } else {
            self.iter().map(|i| i.size()).sum()
        }
    }
}

#[derive(Clone)]
pub enum IteratorArrayIterator<'s, 'a: 's, T, I>
    where T: Readable<'a> + 's,
          I: Iterator<Item=T::Args> + ExactSizeIterator + Clone
{
    Borrowed(Reader<'a>, I),
    Owned(SliceIter<'s, T>),
}

impl<'s, 'a: 's, T, I> Iterator for IteratorArrayIterator<'s, 'a, T, I>
    where T: Readable<'a> + 's,
          I: Iterator<Item=T::Args> + ExactSizeIterator + Clone
{
    type Item = LCow<'s, T>;
    fn next(&mut self) -> Option<Self::Item>
    {
        match *self {
            IteratorArrayIterator::Borrowed(ref mut reader, ref mut args_iter) => {
                if let Some(args) = args_iter.next() {
                    let res = reader.read::<T>(args);
                    Some(LCow::Owned(res))
                } else {
                    None
                }
            },
            IteratorArrayIterator::Owned(ref mut iter) => iter.next().map(LCow::Borrowed),
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

impl<'s, 'a: 's, T, I> ExactSizeIterator for IteratorArrayIterator<'s, 'a, T, I>
    where T: Readable<'a> + 's,
          I: Iterator<Item=T::Args> + ExactSizeIterator + Clone
{
    fn len(&self) -> usize
    {
        match *self {
            IteratorArrayIterator::Borrowed(_, ref args_iter) => args_iter.len(),
            IteratorArrayIterator::Owned(ref iter) => iter.len(),
        }
    }
}

impl<'a, T, I> Writable for IteratorArray<'a, T, I>
    where T: Readable<'a> + Writable,
          I: Iterator<Item=T::Args> + ExactSizeIterator + Clone
{
    fn write<W: io::Write>(&self, writer: &mut W) -> io::Result<()>
    {
        for i in self.iter() {
            i.write(writer)?
        }
        Ok(())
    }
}

impl<'a, T, I> fmt::Debug for IteratorArray<'a, T, I>
    where T: Readable<'a> + fmt::Debug,
          I: Iterator<Item=T::Args> + ExactSizeIterator + Clone
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        fmt::Debug::fmt(&self.iter().collect::<Vec<_>>(), f)
    }
}

impl<'a, T, I> From<Vec<T>> for IteratorArray<'a, T, I>
    where T: Readable<'a>,
          I: Iterator<Item=T::Args> + ExactSizeIterator + Clone
{
    fn from(vec: Vec<T>) -> IteratorArray<'a, T, I>
    {
        IteratorArray::Owned(vec)
    }
}

