//! Utilities for padding for alignment
use crate::{
    read_only_array::RoArray,
    reader::{Readable, Reader, ReaderEx},
    writer::{Writable, Writer},
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


pub fn pad_bytes<R>(align_to: usize, n: usize) -> RoArray<R, u8>
    where R: Reader + From<&'static [u8]>,
{
    R::from(&BYTES_00).read((pad_bytes_count(align_to, n), ()))
        .unwrap_or_else(|_| unreachable!())
}

pub fn pad_bytes_ff<R>(align_to: usize, n: usize) -> RoArray<R, u8>
    where R: Reader + From<&'static [u8]>,
{
    R::from(&BYTES_FF).read((pad_bytes_count(align_to, n), ()))
        .unwrap_or_else(|_| unreachable!())
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct PaddingBlackhole(pub usize);

impl<R: Reader> Readable<R> for PaddingBlackhole
{
    type Args = usize;
    fn read_from(reader: &mut R, i: Self::Args) -> Result<Self, R::Error>
    {
        reader.advance(i)?;
        Ok(PaddingBlackhole(i))
    }

    fn size(&self) -> Result<usize, R::Error>
    {
        Ok(self.0)
    }
}

impl<W: Writer> Writable<W> for PaddingBlackhole
{
    fn write_to(&self, w: &mut W) -> Result<u64, W::Error>
    {
        w.write_bytes(&BYTES_00[..self.0])?;
        Ok(self.0 as u64)
    }
}
