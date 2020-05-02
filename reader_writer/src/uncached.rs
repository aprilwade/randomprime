use std::{
    io,
    fmt::{Debug, Formatter, Error},
};

use crate::{
    lcow::LCow,
    reader::{Reader, Readable},
    writer::Writable,
};

pub enum Uncached<'r, T>
    where T: Readable<'r>
{
    Borrowed(Reader<'r>, T::Args),
    Owned(Box<T>),
}

impl<'r, T> Uncached<'r, T>
    where T: Readable<'r>,
          T::Args: Clone,
{
    pub fn get(&self) -> LCow<T>
    {
        match self {
            Self::Borrowed(reader, args) => LCow::Owned(reader.clone().read(args.clone())),
            Self::Owned(t) => LCow::Borrowed(t),
        }
    }

    pub fn get_mut(&mut self) -> &mut T
    {
        match self {
            Self::Borrowed(reader, args) => {
                *self = Uncached::Owned(Box::new(reader.clone().read(args.clone())));
                self.get_mut()
            }
            Self::Owned(t) => t,
        }
    }
}

impl<'r, T> Readable<'r> for Uncached<'r, T>
    where T: Readable<'r>,
          T::Args: Clone,
{
    type Args = T::Args;
    fn read_from(reader: &mut Reader<'r>, args: Self::Args) -> Self
    {
        let start_reader = reader.clone();
        let _ = <T as Readable>::read_from(reader, args.clone());
        let size = start_reader.len() - reader.len();

        Uncached::Borrowed(start_reader.truncated(size), args)
    }

    fn size(&self) -> usize
    {
        match self {
            Self::Borrowed(reader, _) => reader.len(),
            Self::Owned(t) => t.size(),
        }
    }
}

impl<'r, T> Debug for Uncached<'r, T>
    where T: Readable<'r> + Debug,
          T::Args: Clone,
{
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), Error>
    {
        Debug::fmt(&self.get(), formatter)
    }
}

impl<'r, T> Clone for Uncached<'r, T>
    where T: Readable<'r> + Clone,
          T::Args: Clone,
{
    fn clone(&self) -> Self
    {
        match self {
            Uncached::Borrowed(reader, args) => Uncached::Borrowed(reader.clone(), args.clone()),
            Uncached::Owned(t) => Uncached::Owned(t.clone()),
        }
    }
}

impl<'r, T> Writable for Uncached<'r, T>
    where T: Readable<'r> + Writable,
          T::Args: Clone,
{
    fn write_to<W: io::Write>(&self, writer: &mut W) -> io::Result<u64>
    {
        match self {
            Uncached::Borrowed(reader, _) => {
                writer.write_all(&reader)?;
                Ok(reader.len() as u64)
            },
            Uncached::Owned(t) => t.write_to(writer),
        }
    }
}
