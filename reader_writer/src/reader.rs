use std::io;
use std::fmt::{Debug, Formatter, Error};
use std::ops::{Deref, DerefMut};

use crate::writer::Writable;

#[derive(Clone)]
pub struct Reader<'a>(&'a [u8]);


impl<'a> Deref for Reader<'a>
{
    type Target = &'a [u8];
    fn deref(&self) -> &Self::Target
    {
        &self.0
    }
}

impl<'a> DerefMut for Reader<'a>
{
    fn deref_mut(&mut self) -> &mut Self::Target
    {
        &mut self.0
    }
}

impl<'a> Debug for Reader<'a>
{
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), Error>
    {
        let ptr = self.0.as_ptr() as *const _ as usize;
        formatter.write_fmt(format_args!("Reader(0x{:x})", ptr))
    }
}

impl <'a> Reader<'a>
{
    pub fn new(data: &'a [u8]) -> Reader<'a>
    {
        Reader(data)
    }

    pub fn dummy() -> Reader<'a>
    {
        Reader(&[])
    }

    pub fn read<T>(&mut self, args: T::Args) -> T
        where T : Readable<'a>
    {
        let res = T::read(self.clone(), args);
        *self = res.1;
        res.0
    }

    pub fn advance(&mut self, len: usize)
    {
        self.0 = self.0.split_at(len).1
    }

    pub fn offset(&self, len: usize) -> Reader<'a>
    {
        Reader(self.0.split_at(len).1)
    }

    pub fn truncate(&mut self, len: usize)
    {
        *self = Reader(&self.0[0..len])
    }

    pub fn truncated(&self, len: usize) -> Reader<'a>
    {
        Reader(&self.0[0..len])
    }
}

impl<'a> Readable<'a> for Reader<'a>
{
    type Args = ();
    fn read(reader: Reader<'a>, (): ()) -> (Self, Reader<'a>)
    {
        (reader.clone(), reader)
    }

    fn fixed_size() -> Option<usize>
    {
        Some(0)
    }
}


impl<'a> Writable for Reader<'a>
{
    fn write<W: io::Write>(&self, _: &mut W) -> io::Result<()>
    {
        Ok(())
    }
}

pub trait Readable<'a> : Sized
{
    type Args;
    fn read(reader: Reader<'a>, args: Self::Args) -> (Self, Reader<'a>);

    fn size(&self) -> usize
    {
        Self::fixed_size().expect("Expected fixed size")
    }

    fn fixed_size() -> Option<usize>
    {
        None
    }
}
