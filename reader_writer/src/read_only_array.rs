
use std::fmt;
use std::io::Write;

use reader::{Reader, Readable};
use writer::Writable;

/// Read only array
#[derive(Clone)]
pub struct RoArray<'a, T>
    where T: Readable<'a>,
          T::Args: Clone,
{
    t_args: T::Args,
    length: usize,
    data_start: Reader<'a>,
}


impl<'a, T> RoArray<'a, T>
    where T: Readable<'a>,
          T::Args: Clone,
{
    #[inline]
    pub fn len(&self) -> usize
    {
        self.length
    }

    #[inline]
    pub fn iter(&self) -> RoArrayIter<'a, T>
    {
        RoArrayIter {
            t_args: self.t_args.clone(),
            length: self.length,
            data_start: self.data_start.clone(),
        }
    }

    pub fn split_off(&mut self, at: usize) -> RoArray<'a, T>
    {
        if at > self.length {
            panic!("`at` ({}) cannot be > the array's length ({}).", at, self.length)
        };
        let new_len = self.length - at;
        // Shorten self to the new length
        self.length = at;
        // self is now the new length, so calculate its new size
        let new_size = self.size();
        RoArray {
            t_args: self.t_args.clone(),
            length: new_len,
            data_start: self.data_start.offset(new_size),
        }
    }

    #[inline]
    pub fn get(&self, at: usize) -> Option<T>
    {
        let fixed_size = T::fixed_size().expect(
                "Array::get should only be called for Ts that are fixed size.");
        if at >= self.length {
            None
        } else {
            Some(self.data_start.offset(at * fixed_size).read(self.t_args.clone()))
        }
    }
}

impl<'a, T> Readable<'a> for RoArray<'a, T>
    where T: Readable<'a>,
          T::Args: Clone,
{
    type Args = (usize, T::Args);

    #[inline]
    fn read(reader: Reader<'a>, (size, args): Self::Args) -> (Self, Reader<'a>)
    {
        let array = RoArray {
            t_args: args,
            length: size,
            data_start: reader.clone(),
        };
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

impl<'a, T> fmt::Debug for RoArray<'a, T>
    where T: Readable<'a> + fmt::Debug,
          T::Args: Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        write!(f, "{:?}", self.iter().collect::<Vec<_>>())
    }
}


#[derive(Clone, Debug)]
pub struct RoArrayIter<'a, T>
    where T: Readable<'a>,
          T::Args: Clone,
{
    data_start: Reader<'a>,
    length: usize,
    t_args: T::Args,
}

impl<'a, T> Iterator for RoArrayIter<'a, T>
    where T: Readable<'a>,
          T::Args: Clone,
{
    type Item = T;
    #[inline]
    fn next(&mut self) -> Option<Self::Item>
    {
        if self.length == 0 {
            None
        } else {
            self.length -= 1;
            Some(self.data_start.read::<T>(self.t_args.clone()))
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>)
    {
        (self.length, Some(self.length))
    }
}

impl<'a, T> ExactSizeIterator for RoArrayIter<'a, T>
    where T: Readable<'a>,
          T::Args: Clone,
{
    #[inline]
    fn len(&self) -> usize
    {
        self.length
    }
}


impl<'a, T> Writable for RoArray<'a, T>
    where T: Readable<'a> + Writable,
          T::Args: Clone,
{
    #[inline]
    fn write<W: Write>(&self, writer: &mut W)
    {
        // TODO: Could this be done more efficently by using the length component of
        //       the reader?
        let len = self.size();
        writer.write(&(*self.data_start)[0..len]).unwrap();
    }
}
