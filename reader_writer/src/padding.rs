//! Utilities for padding for alignment
use std::io;
use crate::{
    read_only_array::RoArray,
    reader::{Readable, Reader},
    writer::Writable,
};


pub fn align_byte_count(align_to: usize, n: usize) -> usize
{
    // TODO: Assert align_to is a power of 2?
    let adjust = align_to - 1;
    (n + adjust) & (usize::max_value() - adjust)
}

pub fn pad_bytes_count(align_to: usize, n: usize) -> usize
{
    align_byte_count(align_to, n) - n
}

static BYTES_00: [u8; 32] = [0; 32];
static BYTES_FF: [u8; 32] = [0; 32];


pub fn pad_bytes<'r>(align_to: usize, n: usize) -> RoArray<'r, u8>
{
    Reader::new(&BYTES_00).read((pad_bytes_count(align_to, n), ()))
}

pub fn pad_bytes_ff<'r>(align_to: usize, n: usize) -> RoArray<'r, u8>
{
    Reader::new(&BYTES_FF).read((pad_bytes_count(align_to, n), ()))
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct PaddingBlackhole(pub usize);

impl<'r> Readable<'r> for PaddingBlackhole
{
    type Args = usize;
    fn read_from(reader: &mut Reader<'r>, i: Self::Args) -> Self
    {
        reader.advance(i);
        PaddingBlackhole(i)
    }

    fn size(&self) -> usize
    {
        self.0
    }
}

impl Writable for PaddingBlackhole
{
    fn write_to<W: io::Write>(&self, w: &mut W) -> io::Result<u64>
    {
        w.write_all(&BYTES_00[..self.0])?;
        Ok(self.0 as u64)
    }
}
