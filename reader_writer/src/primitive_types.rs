

use std::ffi;
use std::fmt;
use std::io;
use std::mem;
use std::borrow::Cow;
use std::marker::PhantomData;

use byteorder::{ReadBytesExt, WriteBytesExt, BigEndian};

use crate::reader::{Readable, Reader};
use crate::writer::Writable;

macro_rules! define_arith_readable {
    ( $(($T: ty, $rf: ident, $wf: ident)),* ) => {
        $(
            impl<'r> Readable<'r> for $T
            {
                type Args = ();
                fn read_from(reader: &mut Reader<'r>, (): ()) -> $T
                {
                    let res = reader.$rf::<BigEndian>();
                    res.unwrap()
                }

                fn fixed_size() -> Option<usize>
                {
                    Some(mem::size_of::<$T>())
                }
            }
            impl<'r> Writable for $T
            {
                fn write_to<W: io::Write>(&self, writer: &mut W) -> io::Result<u64>
                {
                    writer.$wf::<BigEndian>(*self)?;
                    Ok(mem::size_of::<$T>() as u64)
                }
            }
        )*
    }
}

macro_rules! define_byte_readable {
    ( $(($T: ty, $rf: ident, $wf: ident)),* ) => {
        $(
            impl<'r> Readable<'r> for $T
            {
                type Args = ();
                fn read_from(reader: &mut Reader<'r>, (): ()) -> $T
                {
                    let res = reader.$rf();
                    res.unwrap()
                }

                fn fixed_size() -> Option<usize>
                {
                    Some(mem::size_of::<$T>())
                }
            }
            impl<'r> Writable for $T
            {
                fn write_to<W: io::Write>(&self, writer: &mut W) -> io::Result<u64>
                {
                    writer.$wf(*self)?;
                    Ok(mem::size_of::<$T>() as u64)
                }
            }
        )*
    }
}


define_byte_readable!((u8, read_u8, write_u8), (i8, read_i8, write_i8));
define_arith_readable!((u16, read_u16, write_u16), (i16, read_i16, write_i16));
define_arith_readable!((u32, read_u32, write_u32), (i32, read_i32, write_i32));
define_arith_readable!((u64, read_u64, write_u64), (i64, read_i64, write_i64));
define_arith_readable!((f32, read_f32, write_f32), (f64, read_f64, write_f64));



#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FourCC([u8; 4]);

impl FourCC
{
    pub fn new(val: u32) -> FourCC
    {
        let mut data = [0u8; 4];
        (&mut data as &mut [u8]).write_u32::<BigEndian>(val).unwrap();
        FourCC(data)
    }

    pub const fn from_bytes(bytes: &[u8; 4]) -> FourCC
    {
        FourCC(*bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; 4]
    {
        &self.0
    }

    pub fn to_u32(&self) -> u32
    {
        (&self.0 as &[u8]).read_u32::<BigEndian>().unwrap()
    }
}

impl<'r> Readable<'r> for FourCC
{
    type Args = ();
    fn read_from(reader: &mut Reader<'r>, (): ()) -> FourCC
    {
        // TODO: Verify ordering
        let res = [reader.read(()), reader.read(()),
                   reader.read(()), reader.read(())];
        FourCC::from_bytes(&res)
    }

    fn fixed_size() -> Option<usize>
    {
        Some(4)
    }
}

impl Writable for FourCC
{
    fn write_to<W: io::Write>(&self, writer: &mut W) -> io::Result<u64>
    {
        writer.write_all(&self.0)?;
        Ok(4)
    }
}

impl fmt::Display for FourCC
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        write!(f, "{}{}{}{}", self.0[0] as char, self.0[1] as char,
                              self.0[2] as char, self.0[3] as char)
    }
}

impl fmt::Debug for FourCC
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        write!(f, "{}{}{}{}", self.0[0] as char, self.0[1] as char,
                              self.0[2] as char, self.0[3] as char)
    }
}

impl<'r> From<&'r [u8; 4]> for FourCC
{
    fn from(this: &'r [u8; 4]) -> FourCC
    {
        FourCC::from_bytes(this)
    }
}


impl<'r, T> Readable<'r> for Option<T>
    where T: Readable<'r>
{
    type Args = Option<T::Args>;
    fn read_from(reader: &mut Reader<'r>, args: Self::Args) -> Option<T>
    {
        if let Some(args) = args {
            let res = reader.read(args);
            Some(res)
        } else {
            None
        }
    }

    fn size(&self) -> usize
    {
        self.as_ref().map(|i| i.size()).unwrap_or(0)
    }
}

impl<T> Writable for Option<T>
    where T: Writable
{
    fn write_to<W: io::Write>(&self, writer: &mut W) -> io::Result<u64>
    {
        if let Some(ref i) = *self {
            i.write_to(writer)
        } else {
            Ok(0)
        }
    }
}

impl<'r, T> Readable<'r> for Box<T>
    where T: Readable<'r>
{
    type Args = T::Args;
    fn read_from(reader: &mut Reader<'r>, args: T::Args) -> Box<T>
    {
        Box::new(reader.read(args))
    }

    fn size(&self) -> usize
    {
        <T as Readable>::size(&self)
    }

    fn fixed_size() -> Option<usize>
    {
        T::fixed_size()
    }
}

impl<T> Writable for Box<T>
    where T: Writable
{
    fn write_to<W: io::Write>(&self, writer: &mut W) -> io::Result<u64>
    {
        (**self).write_to(writer)
    }
}


impl<'r, T> Readable<'r> for PhantomData<T>
{
    type Args = ();
    fn read_from(_reader: &mut Reader<'r>, (): ()) -> Self
    {
        PhantomData
    }

    fn fixed_size() -> Option<usize>
    {
        Some(0)
    }
}

impl<T> Writable for PhantomData<T>
{
    fn write_to<W: io::Write>(&self, _: &mut W) -> io::Result<u64>
    {
        Ok(0)
    }
}

pub type CStr<'r> = Cow<'r, ffi::CStr>;
impl<'r> Readable<'r> for CStr<'r>
{
    type Args = ();
    fn read_from(reader: &mut Reader<'r>, (): ()) -> CStr<'r>
    {
        // XXX A possible optimization would be to use from_bytes_with_nul_unchecked here
        let buf = &(*reader)[0..(reader.iter().position(|&i| i == b'\0').unwrap() + 1)];
        let cstr = Cow::Borrowed(ffi::CStr::from_bytes_with_nul(buf).unwrap());
        let len = cstr.size();
        reader.advance(len);
        cstr
    }

    fn size(&self) -> usize
    {
        self.to_bytes_with_nul().len()
    }
}

impl<'r> Writable for CStr<'r>
{
    fn write_to<W: io::Write>(&self, writer: &mut W) -> io::Result<u64>
    {
        let slice = self.to_bytes_with_nul();
        writer.write_all(slice)?;
        Ok(slice.len() as u64)
    }
}

pub trait CStrConversionExtension
{
    fn as_cstr<'r>(&'r self) -> CStr<'r>;
}

impl CStrConversionExtension for [u8]
{
    fn as_cstr<'r>(&'r self) -> CStr<'r>
    {
        Cow::Borrowed(ffi::CStr::from_bytes_with_nul(self).unwrap())
    }
}
