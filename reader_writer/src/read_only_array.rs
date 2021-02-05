
use std::fmt;

use crate::{
    reader::{Reader, Readable, ReaderEx, copy},
    writer::{Writable, Writer},
    derivable_array_proxy::DerivableFromIterator,
};

/// Read only array
pub struct RoArray<R, T>
    where R: Reader,
          T: Readable<R>,
{
    t_args: T::Args,
    length: usize,
    data_start: R,
}

impl<R, T> Clone for RoArray<R, T>
    where R: Reader + Clone,
          T: Readable<R>,
          T::Args: Clone,
{
    fn clone(&self) -> Self {
        RoArray {
            t_args: self.t_args.clone(),
            length: self.length,
            data_start: self.data_start.clone(),
        }
    }
}

impl<R, T> RoArray<R, T>
    where R: Reader,
          T: Readable<R>,
{
    pub fn len(&self) -> usize
    {
        self.length
    }

    pub fn iter(&self) -> RoArrayIter<R, T>
        where T::Args: Clone,
    {
        RoArrayIter {
            t_args: self.t_args.clone(),
            length: self.length,
            data_start: self.data_start.clone(),
        }
    }

    pub fn split_off(&mut self, at: usize) -> Result<RoArray<R, T>, R::Error>
        where T::Args: Clone
    {
        if at > self.length {
            panic!("`at` ({}) cannot be > the array's length ({}).", at, self.length)
        };
        let right_len = self.length - at;
        // Shorten self to the new length
        self.length = at;
        // self is now the new length, so calculate its new size
        let new_size = T::fixed_size()
            .map(|i| Ok(i * self.length))
            .unwrap_or_else(|| self.iter().try_fold(0, |s, i| Ok(s + i?.size()?)))?;

        let res = RoArray {
            t_args: self.t_args.clone(),
            length: right_len,
            data_start: self.data_start.advance_clone(new_size)?,
        };
        self.data_start.truncate_to(new_size)?;
        Ok(res)
    }

    pub fn get(&self, at: usize) -> Result<Option<T>, R::Error>
        where T::Args: Clone
    {
        let fixed_size = T::fixed_size().expect(
                "Array::get should only be called for Ts that are fixed size.");
        if at >= self.length {
            Ok(None)
        } else {
            Ok(Some(self.data_start.advance_clone(at * fixed_size)?.read(self.t_args.clone())?))
        }
    }

    pub fn data_start(&self) -> &R
    {
        &self.data_start
    }
}

impl<R, T> Readable<R> for RoArray<R, T>
    where R: Reader,
          T: Readable<R>,
          T::Args: Clone,
{
    type Args = (usize, T::Args);

    // TODO: It would be cool to cache the size in the reader's length field.
    fn read_from(reader: &mut R, (length, args): Self::Args) -> Result<Self, R::Error>
    {
        let size = if let Some(fs) = T::fixed_size() {
            fs * length
        } else {
            let mut iter = RoArrayIter::<R, T> {
                t_args: args.clone(),
                length,
                data_start: reader.clone(),
            };
            iter.try_fold(0, |s, i| Ok(s + i?.size()?))?
        };
        let array = RoArray {
            t_args: args,
            length,
            data_start: reader.truncate_clone_to(size)?,
        };
        reader.advance(size)?;
        Ok(array)
    }

    fn size(&self) -> Result<usize, R::Error>
    {
        Ok(self.data_start.len())
    }
}

impl<R, T> fmt::Debug for RoArray<R, T>
    where R: Reader,
          T: Readable<R>,
          T::Args: Clone,
          T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        let res: Result<Vec<_>, _> = self.iter().collect();
        fmt::Debug::fmt(&res.unwrap_or_else(|_| panic!("Error while fmting a RoArray")), f)
    }
}


#[derive(Clone, Debug)]
pub struct RoArrayIter<R, T>
    where R: Reader,
          T: Readable<R>,
{
    data_start: R,
    length: usize,
    t_args: T::Args,
}

impl<R, T> Iterator for RoArrayIter<R, T>
    where R: Reader,
          T: Readable<R>,
          T::Args: Clone,
{
    type Item = Result<T, R::Error>;
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

impl<R, T> ExactSizeIterator for RoArrayIter<R, T>
    where R: Reader,
          T: Readable<R>,
          T::Args: Clone,
{
    fn len(&self) -> usize
    {
        self.length
    }
}


impl<R, W, T> Writable<W> for RoArray<R, T>
    where R: Reader,
          W: Writer,
          T: Readable<R>,
          T::Args: Clone,
          W::Error: From<R::Error>
{
    fn write_to(&self, writer: &mut W) -> Result<u64, W::Error>
    {
        copy(&mut self.data_start.clone(), writer)
    }
}

impl<R, T> DerivableFromIterator for RoArray<R, T>
    where R: Reader,
          T: Readable<R>,
          T::Args: Clone,
{
        type Item = T;
}

#[cfg(test)]
mod tests
{
    use crate::reader::{ReaderEx, SliceReader};
    use super::RoArray;
    #[test]
    fn test_split_off()
    {
        let data = [1, 2, 3, 4, 5];
        let mut reader = SliceReader::<byteorder::LittleEndian>::new(&data);
        let mut array: RoArray<_, u8> = reader.read((5, ())).unwrap();
        let right = array.split_off(2).unwrap();
        assert_eq!(array.iter().collect::<Result<Vec<_>, _>>().unwrap(), [1, 2]);
        assert_eq!(right.iter().collect::<Result<Vec<_>, _>>().unwrap(), [3, 4, 5]);
    }
}
