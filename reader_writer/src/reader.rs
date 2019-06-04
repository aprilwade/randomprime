use std::{
    io,
    fmt::{Debug, Formatter, Error},
    ops::{Deref, DerefMut},
};

use crate::writer::Writable;

#[derive(Clone)]
pub struct Reader<'r>(&'r [u8]);


impl<'r> Deref for Reader<'r>
{
    type Target = &'r [u8];
    fn deref(&self) -> &Self::Target
    {
        &self.0
    }
}

impl<'r> DerefMut for Reader<'r>
{
    fn deref_mut(&mut self) -> &mut Self::Target
    {
        &mut self.0
    }
}

impl<'r> Debug for Reader<'r>
{
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), Error>
    {
        let ptr = self.0.as_ptr() as *const _ as usize;
        formatter.write_fmt(format_args!("Reader(0x{:x})", ptr))
    }
}

impl <'r> Reader<'r>
{
    pub fn new(data: &'r [u8]) -> Reader<'r>
    {
        Reader(data)
    }

    pub fn dummy() -> Reader<'r>
    {
        Reader(&[])
    }

    pub fn read<T>(&mut self, args: T::Args) -> T
        where T : Readable<'r>
    {
        T::read_from(self, args)
    }

    pub fn advance(&mut self, len: usize)
    {
        self.0 = self.0.split_at(len).1
    }

    pub fn offset(&self, len: usize) -> Reader<'r>
    {
        Reader(self.0.split_at(len).1)
    }

    pub fn truncate(&mut self, len: usize)
    {
        *self = Reader(&self.0[0..len])
    }

    pub fn truncated(&self, len: usize) -> Reader<'r>
    {
        Reader(&self.0[0..len])
    }
}

impl<'r> Readable<'r> for Reader<'r>
{
    type Args = ();
    fn read_from(reader: &mut Reader<'r>, (): ()) -> Self
    {
        reader.clone()
    }

    fn fixed_size() -> Option<usize>
    {
        Some(0)
    }
}


impl<'r> Writable for Reader<'r>
{
    fn write_to<W: io::Write>(&self, _: &mut W) -> io::Result<u64>
    {
        Ok(0)
    }
}

pub trait Readable<'r> : Sized
{
    type Args;
    fn read_from(reader: &mut Reader<'r>, args: Self::Args) -> Self;

    fn size(&self) -> usize
    {
        Self::fixed_size().expect("Expected fixed size")
    }

    fn fixed_size() -> Option<usize>
    {
        None
    }
}
