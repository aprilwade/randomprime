use std::{
    fmt,
    io,
    slice::Iter as SliceIter,
};

use crate::{
    lcow::LCow,
    reader::{Reader, Readable},
    writer::Writable,
};


#[derive(Clone)]
pub enum IteratorArray<'r, T, I>
    where T: Readable<'r>,
          I: Iterator<Item=T::Args> + ExactSizeIterator + Clone
{
    Borrowed(Reader<'r>, I),
    Owned(Vec<T>),
}

impl<'r, T, I> IteratorArray<'r, T, I>
    where T: Readable<'r>,
          I: Iterator<Item=T::Args> + ExactSizeIterator + Clone
{
    pub fn len(&self) -> usize
    {
        match *self {
            IteratorArray::Borrowed(_, ref i) => i.len(),
            IteratorArray::Owned(ref vec) => vec.len(),
        }
    }

    pub fn iter<'s>(&'s self) -> IteratorArrayIterator<'s, 'r, T, I>
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

impl<'r, T, I> Readable<'r> for IteratorArray<'r, T, I>
    where T: Readable<'r>,
          I: Iterator<Item=T::Args> + ExactSizeIterator + Clone
{
    type Args = I;
    fn read_from(reader: &mut Reader<'r>, i: I) -> Self
    {
        let res = IteratorArray::Borrowed(reader.clone(), i);
        reader.advance(res.size());
        res
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
pub enum IteratorArrayIterator<'s, 'r: 's, T, I>
    where T: Readable<'r> + 's,
          I: Iterator<Item=T::Args> + ExactSizeIterator + Clone
{
    Borrowed(Reader<'r>, I),
    Owned(SliceIter<'s, T>),
}

impl<'s, 'r: 's, T, I> Iterator for IteratorArrayIterator<'s, 'r, T, I>
    where T: Readable<'r> + 's,
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

impl<'s, 'r: 's, T, I> ExactSizeIterator for IteratorArrayIterator<'s, 'r, T, I>
    where T: Readable<'r> + 's,
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

impl<'r, T, I> Writable for IteratorArray<'r, T, I>
    where T: Readable<'r> + Writable,
          I: Iterator<Item=T::Args> + ExactSizeIterator + Clone
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

impl<'r, T, I> fmt::Debug for IteratorArray<'r, T, I>
    where T: Readable<'r> + fmt::Debug,
          I: Iterator<Item=T::Args> + ExactSizeIterator + Clone
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        fmt::Debug::fmt(&self.iter().collect::<Vec<_>>(), f)
    }
}

impl<'r, T, I> From<Vec<T>> for IteratorArray<'r, T, I>
    where T: Readable<'r>,
          I: Iterator<Item=T::Args> + ExactSizeIterator + Clone
{
    fn from(vec: Vec<T>) -> IteratorArray<'r, T, I>
    {
        IteratorArray::Owned(vec)
    }
}

