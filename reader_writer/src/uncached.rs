use std::fmt::{Debug, Formatter, Error};

use crate::{
    lcow::LCow,
    reader::{copy, Reader, Readable, ReaderEx},
    writer::{Writable, Writer},
};

pub enum Uncached<R, T>
    where R: Reader,
          T: Readable<R>
{
    Borrowed(R, T::Args),
    Owned(Box<T>),
}

impl<R, T> Uncached<R, T>
    where R: Reader,
          T: Readable<R>,
          T::Args: Clone,
{
    pub fn get(&self) -> Result<LCow<T>, R::Error>
    {
        match self {
            Self::Borrowed(reader, args) => Ok(LCow::Owned(reader.clone().read(args.clone())?)),
            Self::Owned(t) => Ok(LCow::Borrowed(t)),
        }
    }

    pub fn get_mut(&mut self) -> Result<&mut T, R::Error>
    {
        match self {
            Self::Borrowed(reader, args) => {
                *self = Uncached::Owned(Box::new(reader.read(args.clone())?));
                self.get_mut()
            }
            Self::Owned(ref mut t) => Ok(t),
        }
    }
}

impl<R, T> Readable<R> for Uncached<R, T>
    where R: Reader,
          T: Readable<R>,
          T::Args: Clone,
{
    type Args = T::Args;
    fn read_from(reader: &mut R, args: Self::Args) -> Result<Self, R::Error>
    {
        let mut start_reader = reader.clone();
        let _ = <T as Readable<R>>::read_from(reader, args.clone())?;
        let size = start_reader.len() - reader.len();

        start_reader.truncate_to(size)?;
        Ok(Uncached::Borrowed(start_reader, args))
    }

    fn size(&self) -> Result<usize, R::Error>
    {
        match self {
            Self::Borrowed(reader, _) => Ok(reader.len()),
            Self::Owned(t) => t.size(),
        }
    }
}

impl<R, T> Debug for Uncached<R, T>
    where R: Reader,
          T: Readable<R> + Debug,
          T::Args: Clone,
{
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), Error>
    {
        Debug::fmt(
            &self.get().unwrap_or_else(|_| panic!("Error while formatting Uncached")),
            formatter
        )
    }
}

impl<R, T> Clone for Uncached<R, T>
    where R: Reader,
          T: Readable<R> + Clone,
          T::Args: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Uncached::Borrowed(reader, args) => Uncached::Borrowed(reader.clone(), args.clone()),
            Uncached::Owned(t) => Uncached::Owned(t.clone()),
        }
    }
}

impl<R, W, T> Writable<W> for Uncached<R, T>
    where R: Reader,
          W: Writer,
          W::Error: From<R::Error>,
          T: Readable<R> + Writable<W>,
          T::Args: Clone,
{
    fn write_to(&self, writer: &mut W) -> Result<u64, W::Error>
    {
        match self {
            Uncached::Borrowed(reader, _) => {
                copy(&mut reader.clone(), writer)?;
                Ok(reader.len() as u64)
            },
            Uncached::Owned(t) => t.write_to(writer),
        }
    }
}
