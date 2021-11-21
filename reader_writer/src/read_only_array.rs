
use std::{
    fmt,
    io,
};

use crate::{
    reader::{Reader, Readable},
    writer::Writable,
    derivable_array_proxy::DerivableFromIterator,
};

/// Read only array
#[derive(Clone)]
pub struct RoArray<'r, T>
    where T: Readable<'r>,
          T::Args: Clone,
{
    t_args: T::Args,
    length: usize,
    data_start: Reader<'r>,
}

impl<'r, T> RoArray<'r, T>
    where T: Readable<'r>,
          T::Args: Clone,
{
    pub fn len(&self) -> usize
    {
        self.length
    }

    pub fn iter(&self) -> RoArrayIter<'r, T>
    {
        RoArrayIter {
            t_args: self.t_args.clone(),
            length: self.length,
            data_start: self.data_start.clone(),
        }
    }

    pub fn split_off(&mut self, at: usize) -> RoArray<'r, T>
    {
        if at > self.length {
            panic!("`at` ({}) cannot be > the array's length ({}).", at, self.length)
        };
        let right_len = self.length - at;
        // Shorten self to the new length
        self.length = at;
        // self is now the new length, so calculate its new size
        let new_size = T::fixed_size()
            .map(|i| i * self.length)
            .unwrap_or_else(|| self.iter().fold(0, |s, i| s + i.size()));

        let res = RoArray {
            t_args: self.t_args.clone(),
            length: right_len,
            data_start: self.data_start.offset(new_size),
        };
        self.data_start.truncate(new_size);
        res
    }

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

    pub fn data_start(&self) -> Reader<'r>
    {
        self.data_start.clone()
    }
}

impl<'r, T> Readable<'r> for RoArray<'r, T>
    where T: Readable<'r>,
          T::Args: Clone,
{
    type Args = (usize, T::Args);

    // TODO: It would be cool to cache the size in the reader's length field.
    fn read_from(reader: &mut Reader<'r>, (length, args): Self::Args) -> Self
    {
        let size = T::fixed_size()
            .map(|i| i * length)
            .unwrap_or_else(|| {
                let iter = RoArrayIter::<T> {
                    t_args: args.clone(),
                    length: length,
                    data_start: reader.clone(),
                };
                iter.fold(0, |s, i| s + i.size())
            });
        let array = RoArray {
            t_args: args,
            length: length,
            data_start: reader.truncated(size),
        };
        reader.advance(size);
        array
    }

    fn size(&self) -> usize
    {
        self.data_start.len()
    }
}

impl<'r, T> fmt::Debug for RoArray<'r, T>
    where T: Readable<'r> + fmt::Debug,
          T::Args: Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        fmt::Debug::fmt(&self.iter().collect::<Vec<_>>(), f)
    }
}


#[derive(Clone, Debug)]
pub struct RoArrayIter<'r, T>
    where T: Readable<'r>,
          T::Args: Clone,
{
    data_start: Reader<'r>,
    length: usize,
    t_args: T::Args,
}

impl<'r, T> Iterator for RoArrayIter<'r, T>
    where T: Readable<'r>,
          T::Args: Clone,
{
    type Item = T;
    fn next(&mut self) -> Option<Self::Item>
    {
        if self.length == 0 {
            None
        } else {
            self.length -= 1;
            Some(self.data_start.read::<T>(self.t_args.clone()))
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>)
    {
        (self.length, Some(self.length))
    }
}

impl<'r, T> ExactSizeIterator for RoArrayIter<'r, T>
    where T: Readable<'r>,
          T::Args: Clone,
{
    fn len(&self) -> usize
    {
        self.length
    }
}


impl<'r, T> Writable for RoArray<'r, T>
    where T: Readable<'r> + Writable,
          T::Args: Clone,
{
    fn write_to<W: io::Write>(&self, writer: &mut W) -> io::Result<u64>
    {
        // TODO: Could this be done more efficently by using the length component of
        //       the reader?
        let len = self.size();
        writer.write_all(&(*self.data_start)[0..len])?;
        Ok(len as u64)
    }
}

impl<'r, T> DerivableFromIterator for RoArray<'r, T>
    where T: Readable<'r>,
          T::Args: Clone,
{
        type Item = T;
}

#[cfg(test)]
mod tests
{
    use crate::{Reader, RoArray};
    #[test]
    fn test_split_off()
    {
        let data = [1, 2, 3, 4, 5];
        let mut reader = Reader::new(&data);
        let mut array: RoArray<u8> = reader.read((5, ()));
        let right = array.split_off(2);
        assert_eq!(array.iter().collect::<Vec<_>>(), [1, 2]);
        assert_eq!(right.iter().collect::<Vec<_>>(), [3, 4, 5]);
    }
}
