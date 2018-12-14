//! Utilities for padding for alignment
use crate::read_only_array::RoArray;
use crate::reader::Reader;


pub fn align_byte_count(align_to: usize, n: usize) -> usize
{
    // TODO: Assert align_to is a power of 2?
    let adjust = align_to - 1;
    ((n + adjust) & (usize::max_value() - adjust))
}

pub fn pad_bytes_count(align_to: usize, n: usize) -> usize
{
    align_byte_count(align_to, n) - n
}

static BYTES_00: [u8; 32] = [0; 32];
static BYTES_FF: [u8; 32] = [0; 32];


pub fn pad_bytes<'a>(align_to: usize, n: usize) -> RoArray<'a, u8>
{
    Reader::new(&BYTES_00).read((pad_bytes_count(align_to, n), ()))
}

pub fn pad_bytes_ff<'a>(align_to: usize, n: usize) -> RoArray<'a, u8>
{
    Reader::new(&BYTES_FF).read((pad_bytes_count(align_to, n), ()))
}
