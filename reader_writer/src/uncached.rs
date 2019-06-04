use std::{
    io,
    fmt::{Debug, Formatter, Error},
};

use crate::{
    reader::{Reader, Readable},
    writer::Writable,
};

pub struct Uncached<'r, T>(Reader<'r>, T::Args)
    where T: Readable<'r>;

impl<'r, T> Uncached<'r, T>
    where T: Readable<'r>,
          T::Args: Clone,
{
    pub fn get(&self) -> T
    {
        self.0.clone().read(self.1.clone())
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

        Uncached(start_reader.truncated(size), args)
    }

    fn size(&self) -> usize
    {
        self.0.len()
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
    where T: Readable<'r>,
          T::Args: Clone,
{
    fn clone(&self) -> Self
    {
        Uncached(self.0.clone(), self.1.clone())
    }
}

impl<'r, T> Writable for Uncached<'r, T>
    where T: Readable<'r>,
          T::Args: Clone,
{
    fn write_to<W: io::Write>(&self, writer: &mut W) -> io::Result<u64>
    {
        writer.write_all(&self.0)?;
        Ok(self.0.len() as u64)
    }
}
