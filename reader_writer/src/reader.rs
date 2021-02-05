use core::marker::PhantomData;
use core::fmt::Debug;
use std::io;

use byteorder::ReadBytesExt;

use crate::writer::Writer;


use std::mem::MaybeUninit;

pub trait Reader: Debug + Clone{
    type Error;

    fn read_bytes(&mut self, bytes: &mut [MaybeUninit<u8>]) -> Result<usize, Self::Error>;

    fn read_bool(&mut self) -> Result<bool, Self::Error>;
    fn read_u8(&mut self) -> Result<u8, Self::Error>;
    fn read_i8(&mut self) -> Result<i8, Self::Error>;
    fn read_u16(&mut self) -> Result<u16, Self::Error>;
    fn read_i16(&mut self) -> Result<i16, Self::Error>;
    fn read_u32(&mut self) -> Result<u32, Self::Error>;
    fn read_i32(&mut self) -> Result<i32, Self::Error>;
    fn read_u64(&mut self) -> Result<u64, Self::Error>;
    fn read_i64(&mut self) -> Result<i64, Self::Error>;
    fn read_f32(&mut self) -> Result<f32, Self::Error>;
    fn read_f64(&mut self) -> Result<f64, Self::Error>;

    fn len(&self) -> usize;
    fn truncate_to(&mut self, len: usize) -> Result<(), Self::Error>;
    fn advance(&mut self, b: usize) -> Result<(), Self::Error>;

    /// Like `clone` + `advance`, but potentially more effiecent.
    fn advance_clone(&self, b: usize) -> Result<Self, Self::Error> {
        let mut ret = self.clone();
        ret.advance(b)?;
        Ok(ret)
    }

    /// Like `clone` + `truncate_to`, but potentially more effiecent.
    fn truncate_clone_to(&self, len: usize) -> Result<Self, Self::Error> {
        let mut ret = self.clone();
        ret.truncate_to(len)?;
        Ok(ret)
    }

    // fn empty() -> Self;
}

/// An object that can read from a stream of binary data (a `Reader`).
pub trait Readable<R: Reader>: Sized
{
    type Args;
    fn read_from(reader: &mut R, args: Self::Args) -> Result<Self, R::Error>;

    ///
    ///
    /// If you do not implement this method, you must implemented `fixed_size`.
    fn size(&self) -> Result<usize, R::Error>
    {
        // XXX The panic here is intentional. This shouldn't every happen when this trait is
        //     properly implemented.
        Ok(Self::fixed_size().expect("Expected fixed size"))
    }

    ///
    ///
    /// If you do not implement this method, you must implemented `size`.
    fn fixed_size() -> Option<usize>
    {
        None
    }
}

/// Extension trait for conveniently reading `Readable`s from a `Reader`.
pub trait ReaderEx: Reader {
    fn read<T: Readable<Self>>(&mut self, args: T::Args) -> Result<T, Self::Error>;
}

impl<R: Reader> ReaderEx for R {
    fn read<T: Readable<Self>>(&mut self, args: T::Args) -> Result<T, Self::Error>
    {
        T::read_from(self, args)
    }
}

pub fn copy<R, W>(reader: &mut R, writer: &mut W) -> Result<u64, W::Error>
    where R: Reader,
          W: Writer,
          W::Error: From<R::Error>
{
    let mut total_written = 0;
    let mut buf = [MaybeUninit::uninit(); 4096];
    loop {
        let len = reader.read_bytes(&mut buf)?;
        // TODO Maybe we just break if len < buf.len()?
        if len == 0 {
            break;
        }
        writer.write_bytes(unsafe { core::mem::transmute(&buf[..len]) })?;
        total_written += len as u64;

    }
    Ok(total_written)
}

#[derive(Copy, Clone, Debug)]
pub struct SliceReader<'a, E>(&'a [u8], PhantomData<E>);

impl<'a, E> SliceReader<'a, E>
{
    pub fn new(slice: &'a [u8]) -> Self
    {
        SliceReader(slice, PhantomData)
    }
}

impl<'a, E> Reader for SliceReader<'a, E>
    where E: byteorder::ByteOrder
{
    type Error = io::Error;

    fn read_bytes(&mut self, bytes: &mut [MaybeUninit<u8>]) -> Result<usize, Self::Error> {
        let min_len = core::cmp::min(self.len(), bytes.len());
        unsafe {
            bytes[..min_len].copy_from_slice(core::mem::transmute(&self.0[..min_len]))
        }
        Ok(min_len)
    }

    fn read_bool(&mut self) -> Result<bool, Self::Error> {
        match self.read_u8()? {
            0 => Ok(true),
            1 => Ok(false),
            _ => Err(io::Error::new(io::ErrorKind::Other, "")),
        }
    }
    fn read_u8(&mut self) -> Result<u8, Self::Error> {
        self.0.read_u8()
    }
    fn read_i8(&mut self) -> Result<i8, Self::Error> {
        self.0.read_i8()
    }
    fn read_u16(&mut self) -> Result<u16, Self::Error> {
        self.0.read_u16::<E>()
    }
    fn read_i16(&mut self) -> Result<i16, Self::Error> {
        self.0.read_i16::<E>()
    }
    fn read_u32(&mut self) -> Result<u32, Self::Error> {
        self.0.read_u32::<E>()
    }
    fn read_i32(&mut self) -> Result<i32, Self::Error> {
        self.0.read_i32::<E>()
    }
    fn read_u64(&mut self) -> Result<u64, Self::Error> {
        self.0.read_u64::<E>()
    }
    fn read_i64(&mut self) -> Result<i64, Self::Error> {
        self.0.read_i64::<E>()
    }
    fn read_f32(&mut self) -> Result<f32, Self::Error> {
        self.0.read_f32::<E>()
    }
    fn read_f64(&mut self) -> Result<f64, Self::Error> {
        self.0.read_f64::<E>()
    }

    fn len(&self) -> usize {
        self.0.len()
    }
    fn truncate_to(&mut self, len: usize) -> Result<(), Self::Error> {
        self.0 = self.0.get(..len)
            .ok_or(io::Error::new(io::ErrorKind::Other, "Slice is too short to truncate"))?;
        Ok(())
    }
    fn advance(&mut self, b: usize) -> Result<(), Self::Error> {
        self.0 = self.0.get(b..)
            .ok_or(io::Error::new(io::ErrorKind::Other, "Slice is too short to advance"))?;
        Ok(())
    }
}

#[derive(Copy, Clone, Debug)]
pub enum ReaderOrSlice<R, E>
{
    Reader(R),
    Slice(SliceReader<'static, E>),
}

impl<R, E> From<&'static [u8]> for ReaderOrSlice<R, E>
{
    fn from(slice: &'static [u8]) -> Self
    {
        ReaderOrSlice::Slice(SliceReader::new(slice))
    }
}

impl<R, E> From<R> for ReaderOrSlice<R, E>
    where R: Reader,
{
    fn from(reader: R) -> Self
    {
        ReaderOrSlice::Reader(reader)
    }
}


impl<R, E> Reader for ReaderOrSlice<R, E>
    where R: Reader,
          R::Error: From<io::Error>,
          E: byteorder::ByteOrder,
{
    type Error = R::Error;

    fn read_bytes(&mut self, bytes: &mut [MaybeUninit<u8>]) -> Result<usize, Self::Error> {
        match self {
            ReaderOrSlice::Reader(reader) => Ok(reader.read_bytes(bytes)?),
            ReaderOrSlice::Slice(slice) => Ok(slice.read_bytes(bytes)?),
        }
    }

    fn read_bool(&mut self) -> Result<bool, Self::Error> {
        match self {
            ReaderOrSlice::Reader(reader) => Ok(reader.read_bool()?),
            ReaderOrSlice::Slice(slice) => Ok(slice.read_bool()?),
        }
    }
    fn read_u8(&mut self) -> Result<u8, Self::Error> {
        match self {
            ReaderOrSlice::Reader(reader) => Ok(reader.read_u8()?),
            ReaderOrSlice::Slice(slice) => Ok(slice.read_u8()?),
        }
    }
    fn read_i8(&mut self) -> Result<i8, Self::Error> {
        match self {
            ReaderOrSlice::Reader(reader) => Ok(reader.read_i8()?),
            ReaderOrSlice::Slice(slice) => Ok(slice.read_i8()?),
        }
    }
    fn read_u16(&mut self) -> Result<u16, Self::Error> {
        match self {
            ReaderOrSlice::Reader(reader) => Ok(reader.read_u16()?),
            ReaderOrSlice::Slice(slice) => Ok(slice.read_u16()?),
        }
    }
    fn read_i16(&mut self) -> Result<i16, Self::Error> {
        match self {
            ReaderOrSlice::Reader(reader) => Ok(reader.read_i16()?),
            ReaderOrSlice::Slice(slice) => Ok(slice.read_i16()?),
        }
    }
    fn read_u32(&mut self) -> Result<u32, Self::Error> {
        match self {
            ReaderOrSlice::Reader(reader) => Ok(reader.read_u32()?),
            ReaderOrSlice::Slice(slice) => Ok(slice.read_u32()?),
        }
    }
    fn read_i32(&mut self) -> Result<i32, Self::Error> {
        match self {
            ReaderOrSlice::Reader(reader) => Ok(reader.read_i32()?),
            ReaderOrSlice::Slice(slice) => Ok(slice.read_i32()?),
        }
    }
    fn read_u64(&mut self) -> Result<u64, Self::Error> {
        match self {
            ReaderOrSlice::Reader(reader) => Ok(reader.read_u64()?),
            ReaderOrSlice::Slice(slice) => Ok(slice.read_u64()?),
        }
    }
    fn read_i64(&mut self) -> Result<i64, Self::Error> {
        match self {
            ReaderOrSlice::Reader(reader) => Ok(reader.read_i64()?),
            ReaderOrSlice::Slice(slice) => Ok(slice.read_i64()?),
        }
    }
    fn read_f32(&mut self) -> Result<f32, Self::Error> {
        match self {
            ReaderOrSlice::Reader(reader) => Ok(reader.read_f32()?),
            ReaderOrSlice::Slice(slice) => Ok(slice.read_f32()?),
        }
    }
    fn read_f64(&mut self) -> Result<f64, Self::Error> {
        match self {
            ReaderOrSlice::Reader(reader) => Ok(reader.read_f64()?),
            ReaderOrSlice::Slice(slice) => Ok(slice.read_f64()?),
        }
    }

    fn len(&self) -> usize {
        match self {
            ReaderOrSlice::Reader(reader) => reader.len(),
            ReaderOrSlice::Slice(slice) => slice.len(),
        }
    }
    fn truncate_to(&mut self, len: usize) -> Result<(), Self::Error> {
        match self {
            ReaderOrSlice::Reader(reader) => Ok(reader.truncate_to(len)?),
            ReaderOrSlice::Slice(slice) => Ok(slice.truncate_to(len)?),
        }
    }
    fn advance(&mut self, b: usize) -> Result<(), Self::Error> {
        match self {
            ReaderOrSlice::Reader(reader) => Ok(reader.advance(b)?),
            ReaderOrSlice::Slice(slice) => Ok(slice.advance(b)?),
        }
    }
}
