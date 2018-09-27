use std::io;
use std::fmt::{Debug, Formatter, Error};

use reader::{Reader, Readable};
use writer::Writable;

pub struct Uncached<'a, T>(Reader<'a>, T::Args)
    where T: Readable<'a>;

impl<'a, T> Uncached<'a, T>
    where T: Readable<'a>,
          T::Args: Clone,
{
    pub fn get(&self) -> T
    {
        self.0.clone().read(self.1.clone())
    }
}

impl<'a, T> Readable<'a> for Uncached<'a, T>
    where T: Readable<'a>,
          T::Args: Clone,
{
    type Args = T::Args;
    fn read(reader: Reader<'a>, args: Self::Args) -> (Self, Reader<'a>)
    {
        let start_reader = reader.clone();
        let (_, after_reader) = <T as Readable>::read(reader, args.clone());
        let size = start_reader.len() - after_reader.len();

        let res = Uncached(start_reader.truncated(size), args);
        (res, after_reader)
    }

    fn size(&self) -> usize
    {
        self.0.len()
    }
}

impl<'a, T> Debug for Uncached<'a, T>
    where T: Readable<'a> + Debug,
          T::Args: Clone,
{
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), Error>
    {
        Debug::fmt(&self.get(), formatter)
    }
}

impl<'a, T> Clone for Uncached<'a, T>
    where T: Readable<'a>,
          T::Args: Clone,
{
    fn clone(&self) -> Self
    {
        Uncached(self.0.clone(), self.1.clone())
    }
}

impl<'a, T> Writable for Uncached<'a, T>
    where T: Readable<'a>,
          T::Args: Clone,
{
    fn write<W: io::Write>(&self, writer: &mut W) -> io::Result<()>
    {
        writer.write_all(&self.0)
    }
}
