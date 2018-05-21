

use std::ffi;
use std::fmt;
use std::io;
use std::mem;
use std::borrow::Cow;
use std::marker::PhantomData;

use byteorder::{ReadBytesExt, WriteBytesExt, BigEndian};

use reader::{Readable, Reader};
use writer::Writable;

macro_rules! define_arith_readable {
    ( $(($T: ty, $rf: ident, $wf: ident)),* ) => {
        $(
            impl<'a> Readable<'a> for $T
            {
                type Args = ();
                #[inline]
                fn read(mut reader: Reader<'a>, (): ()) -> ($T, Reader<'a>)
                {
                    let res = reader.$rf::<BigEndian>();
                    (res.unwrap(), reader)
                }

                #[inline]
                fn fixed_size() -> Option<usize>
                {
                    Some(mem::size_of::<$T>())
                }
            }
            impl<'a> Writable for $T
            {
                fn write<W: io::Write>(&self, writer: &mut W) -> io::Result<()>
                {
                    writer.$wf::<BigEndian>(*self)
                }
            }
        )*
    }
}

macro_rules! define_byte_readable {
    ( $(($T: ty, $rf: ident, $wf: ident)),* ) => {
        $(
            impl<'a> Readable<'a> for $T
            {
                type Args = ();
                #[inline]
                fn read(mut reader: Reader<'a>, (): ()) -> ($T, Reader<'a>)
                {
                    let res = reader.$rf();
                    (res.unwrap(), reader)
                }

                #[inline]
                fn fixed_size() -> Option<usize>
                {
                    Some(mem::size_of::<$T>())
                }
            }
            impl<'a> Writable for $T
            {
                fn write<W: io::Write>(&self, writer: &mut W) -> io::Result<()>
                {
                    writer.$wf(*self)
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
    #[inline]
    pub fn new(val: u32) -> FourCC
    {
        let mut data = [0u8; 4];
        (&mut data as &mut [u8]).write_u32::<BigEndian>(val).unwrap();
        FourCC(data)
    }

    #[inline]
    pub fn from_bytes(bytes: &[u8; 4]) -> FourCC
    {
        FourCC(*bytes)
    }

    #[inline]
    pub fn as_bytes(&self) -> &[u8; 4]
    {
        &self.0
    }

    #[inline]
    pub fn to_u32(&self) -> u32
    {
        (&self.0 as &[u8]).read_u32::<BigEndian>().unwrap()
    }
}

impl<'a> Readable<'a> for FourCC
{
    type Args = ();
    #[inline]
    fn read(mut reader: Reader<'a>, (): ()) -> (FourCC, Reader<'a>)
    {
        // TODO: Verify ordering
        let res = [reader.read(()), reader.read(()),
                   reader.read(()), reader.read(())];
        (FourCC::from_bytes(&res), reader)
    }

    #[inline]
    fn fixed_size() -> Option<usize>
    {
        Some(4)
    }
}

impl Writable for FourCC
{
    fn write<W: io::Write>(&self, writer: &mut W) -> io::Result<()>
    {
        writer.write_all(&self.0)
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

impl<'a> From<&'a [u8; 4]> for FourCC
{
    fn from(this: &'a [u8; 4]) -> FourCC
    {
        FourCC::from_bytes(this)
    }
}


impl<'a, T> Readable<'a> for Option<T>
    where T: Readable<'a>
{
    type Args = Option<T::Args>;
    #[inline]
    fn read(mut reader: Reader<'a>, args: Self::Args)
        -> (Option<T>, Reader<'a>)
    {
        if let Some(args) = args {
            let res = reader.read(args);
            (Some(res), reader)
        } else {
            (None, reader)
        }
    }

    #[inline]
    fn size(&self) -> usize
    {
        self.as_ref().map(|i| i.size()).unwrap_or(0)
    }
}

impl<T> Writable for Option<T>
    where T: Writable
{
    fn write<W: io::Write>(&self, writer: &mut W) -> io::Result<()>
    {
        if let Some(ref i) = *self {
            i.write(writer)
        } else {
            Ok(())
        }
    }
}

impl<'a, T> Readable<'a> for Box<T>
    where T: Readable<'a>
{
    type Args = T::Args;
    #[inline]
    fn read(mut reader: Reader<'a>, args: T::Args) -> (Box<T>, Reader<'a>)
    {
        (Box::new(reader.read(args)), reader)
    }

    #[inline]
    fn size(&self) -> usize
    {
        <T as Readable>::size(&self)
    }

    #[inline]
    fn fixed_size() -> Option<usize>
    {
        T::fixed_size()
    }
}

impl<T> Writable for Box<T>
    where T: Writable
{
    fn write<W: io::Write>(&self, writer: &mut W) -> io::Result<()>
    {
        (**self).write(writer)
    }
}


impl<'a, T> Readable<'a> for PhantomData<T>
{
    type Args = ();
    #[inline]
    fn read(reader: Reader<'a>, (): ()) -> (Self, Reader<'a>)
    {
        (PhantomData, reader)
    }

    #[inline]
    fn fixed_size() -> Option<usize>
    {
        Some(0)
    }
}

impl<T> Writable for PhantomData<T>
{
    fn write<W: io::Write>(&self, _: &mut W) -> io::Result<()>
    {
        Ok(())
    }
}

pub type CStr<'a> = Cow<'a, ffi::CStr>;
impl<'a> Readable<'a> for CStr<'a>
{
    type Args = ();
    fn read(reader: Reader<'a>, (): ()) -> (CStr<'a>, Reader<'a>)
    {
        // XXX A possible optimization would be to use from_bytes_with_nul_unchecked here
        let buf = &(*reader)[0..(reader.iter().position(|&i| i == b'\0').unwrap() + 1)];
        let cstr = Cow::Borrowed(ffi::CStr::from_bytes_with_nul(buf).unwrap());
        let len = cstr.size();
        (cstr, reader.offset(len))
    }

    fn size(&self) -> usize
    {
        self.to_bytes_with_nul().len()
    }
}

impl<'a> Writable for CStr<'a>
{
    fn write<W: io::Write>(&self, writer: &mut W) -> io::Result<()>
    {
        writer.write_all(self.to_bytes_with_nul())
    }
}

pub trait CStrConversionExtension
{
    fn as_cstr<'a>(&'a self) -> CStr<'a>;
}

impl CStrConversionExtension for [u8]
{
    fn as_cstr<'a>(&'a self) -> CStr<'a>
    {
        Cow::Borrowed(ffi::CStr::from_bytes_with_nul(b"Trigger_DoorOpen-component\0").unwrap())
    }
}
