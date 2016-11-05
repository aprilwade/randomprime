use std::fmt;
use std::io::Write;
use std::slice::Iter as SliceIter;
use std::slice::IterMut as SliceIterMut;

use imm_cow::ImmCow;
use reader::{Reader, Readable};
use writer::Writable;
use ref_iterable::RefIterable;

/// An array with a non-fixed length.
#[derive(Clone)]
pub enum Array<'a, T>
    where T: Readable<'a>,
          T::Args: Clone,
{
    Borrowed(Reader<'a>, usize, T::Args),
    Owned(Vec<T>),
}


impl<'a, T> Array<'a, T>
    where T: Readable<'a>,
          T::Args: Clone,
{
    #[inline]
    pub fn len(&self) -> usize
    {
        match *self {
            Array::Borrowed(_, len, _) => len,
            Array::Owned(ref vec) => vec.len(),
        }
    }

    #[inline]
    pub fn iter<'s>(&'s self) -> ArrayIterator<'s, 'a, T>
    {
        self.into_iter()
    }

    #[inline]
    pub fn iter_mut<'s>(&'s mut self) -> SliceIterMut<'s, T>
    {
        self.as_mut_vec().iter_mut()
    }

    #[inline]
    pub fn borrowed_iter(&self) -> Option<ArrayBorrowedIterator<'a, T>>
    {
        match *self {
            Array::Borrowed(ref reader, len, ref args) => 
                Some(ArrayBorrowedIterator {
                    reader: reader.clone(),
                    len: len,
                    args: args.clone(),
                }),
            _ => None,
        }
    }

    pub fn get(&self, index: usize) -> Option<ImmCow<T>>
    {
        match *self {
            Array::Borrowed(ref reader, len, ref args) => {
                let fixed_size = T::fixed_size().expect(
                        "Array::get should only be called for Ts that are fixed size.");
                if index >= len {
                    None
                } else {
                    Some(ImmCow::new_owned(reader.offset(index * fixed_size).read(args.clone())))
                }
            },
            Array::Owned(ref vec) => vec.get(index).map(ImmCow::new_borrowed),
        }
    }

    pub fn split_off(&mut self, at: usize) -> Array<'a, T>
    {
        match *self {
            Array::Borrowed(_, _, _) => {
                // This is kind of tortured because we need self to be unborrowed when we
                // call self.size()

                let new_len = self.len() - at;
                // Shorten self to the new length
                match *self {
                    Array::Borrowed(_, ref mut len, _) => *len = at,
                    Array::Owned(_) => unreachable!(),
                };
                // self is now the new length, so calculate its new size
                let new_size = self.size();
                match *self {
                    Array::Borrowed(ref reader, _, ref args)
                        => Array::Borrowed(reader.offset(new_size), new_len, args.clone()),
                    Array::Owned(_) => unreachable!(),
                }
            },
            Array::Owned(ref mut vec) => return Array::Owned(vec.split_off(at)),
        }
    }

    pub fn as_mut_vec(&mut self) -> &mut Vec<T>
    {
        *self = match *self {
            Array::Borrowed(ref mut reader, size, ref args) => {
                let mut vec = Vec::with_capacity(size);
                for _ in 0..size {
                    vec.push(reader.read(args.clone()));
                };
                Array::Owned(vec)
            },
            Array::Owned(ref mut vec) => return vec,
        };
        match *self {
            Array::Owned(ref mut vec) => vec,
            Array::Borrowed(_, _, _) => unreachable!(),
        }
    }
}

impl<'a, T> Readable<'a> for Array<'a, T>
    where T: Readable<'a>,
          T::Args: Clone,
{
    type Args = (usize, T::Args);

    #[inline]
    fn read(reader: Reader<'a>, (size, args): Self::Args) -> (Self, Reader<'a>)
    {
        let array = Array::Borrowed(reader.clone(), size, args);
        let size = array.size();
        (array, reader.offset(size))
    }

    #[inline]
    fn size(&self) -> usize
    {
        T::fixed_size()
            .map(|i| i * self.len())
            .unwrap_or_else(|| self.iter().fold(0, |s, i| s + i.size()))
    }
}

impl<'a, T> fmt::Debug for Array<'a, T>
    where T: Readable<'a> + fmt::Debug,
          T::Args: Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        write!(f, "{:?}", self.iter().collect::<Vec<_>>())
    }
}


#[derive(Clone)]
pub enum ArrayIterator<'s, 'a, T>
    where T: Readable<'a> + 's,
          T::Args: Clone,
{
    Borrowed(ArrayBorrowedIterator<'a, T>),
    Owned(SliceIter<'s, T>),
}

impl<'s, 'a, T> Iterator for ArrayIterator<'s, 'a, T>
    where T: Readable<'a>,
          T::Args: Clone,
{
    type Item = ImmCow<'s, T>;
    #[inline]
    fn next(&mut self) -> Option<Self::Item>
    {
        match *self {
            ArrayIterator::Borrowed(ref mut iter) => iter.next().map(ImmCow::new_owned),
            ArrayIterator::Owned(ref mut iter) => iter.next().map(ImmCow::new_borrowed),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>)
    {
        match *self {
            ArrayIterator::Borrowed(ref iter) => iter.size_hint(),
            ArrayIterator::Owned(ref iter) => iter.size_hint()
        }
    }
}

impl<'s, 'a, T> ExactSizeIterator for ArrayIterator<'s, 'a, T>
    where T: Readable<'a>,
          T::Args: Clone,
{
    fn len(&self) -> usize
    {
        match *self {
            ArrayIterator::Borrowed(ref iter) => iter.len(),
            ArrayIterator::Owned(ref iter) => iter.len()
        }
    }
}

impl<'s, 'a, T: 's> IntoIterator for &'s Array<'a, T>
    where T: Readable<'a>,
          T::Args: Clone,
{
    type Item = ImmCow<'s, T>;
    type IntoIter = ArrayIterator<'s, 'a, T>;
    #[inline]
    fn into_iter(self) -> Self::IntoIter
    {
        match *self {
            Array::Borrowed(ref reader, len, ref args)
                => ArrayIterator::Borrowed(ArrayBorrowedIterator {
                        reader: reader.clone(), 
                        len: len,
                        args: args.clone(),
                }),
            Array::Owned(ref vec) => ArrayIterator::Owned(vec.iter()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ArrayBorrowedIterator<'a, T>
    where T: Readable<'a>,
          T::Args: Clone,
{
    reader: Reader<'a>,
    len: usize,
    args: T::Args,
}

impl<'a, T> Iterator for ArrayBorrowedIterator<'a, T>
    where T: Readable<'a>,
          T::Args: Clone,
{
    type Item = T;
    #[inline]
    fn next(&mut self) -> Option<Self::Item>
    {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            Some(self.reader.read::<T>(self.args.clone()))
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>)
    {
        (self.len, Some(self.len))
    }
}

impl<'a, T> ExactSizeIterator for ArrayBorrowedIterator<'a, T>
    where T: Readable<'a>,
          T::Args: Clone,
{
    fn len(&self) -> usize
    {
        self.len
    }
}

impl<'s, 'a, T> RefIterable<'s, T> for Array<'a, T>
    where T: Readable<'a> + 's,
          T::Args: Clone,
{
    type Item = ImmCow<'s, T>;
    type Iter = ArrayIterator<'s, 'a, T>;
    fn ref_iter(&'s self) -> Self::Iter
    {
        self.iter()
    }
}


impl<'a, T> Writable for Array<'a, T>
    where T: Readable<'a> + Writable,
          T::Args: Clone,
{
    fn write<W: Write>(&self, writer: &mut W)
    {
        match *self {
            Array::Borrowed(ref reader, _, _) => {
                // Just copy the bytes directly
                let len = self.size();
                writer.write(&(*reader)[0..len]).unwrap();
            },
            Array::Owned(ref vec) => {
                for elem in vec {
                    elem.write(writer);
                }
            },
        }
    }
}
