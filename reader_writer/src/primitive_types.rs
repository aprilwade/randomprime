use std::ffi;
use std::fmt;
use std::mem;
use std::borrow::Cow;
use std::marker::PhantomData;

use crate::reader::{Readable, Reader, ReaderEx};
use crate::writer::{Writable, Writer};

macro_rules! define_arith_readable {
    ( $(($T: ty, $rf: ident, $wf: ident)),* ) => {
        $(
            impl<R: Reader> Readable<R> for $T
            {
                type Args = ();
                fn read_from(reader: &mut R, (): ()) -> Result<$T, R::Error>
                {
                    reader.$rf()
                }

                fn fixed_size() -> Option<usize>
                {
                    Some(mem::size_of::<$T>())
                }
            }
            impl<W: Writer> Writable<W> for $T
            {
                fn write_to(&self, writer: &mut W) -> Result<u64, W::Error>
                {
                    writer.$wf(*self)?;
                    Ok(mem::size_of::<$T>() as u64)
                }
            }
        )*
    }
}

macro_rules! define_byte_readable {
    ( $(($T: ty, $rf: ident, $wf: ident)),* ) => {
        $(
            impl<R: Reader> Readable<R> for $T
            {
                type Args = ();
                fn read_from(reader: &mut R, (): ()) -> Result<$T, R::Error>
                {
                    reader.$rf()
                }

                fn fixed_size() -> Option<usize>
                {
                    Some(mem::size_of::<$T>())
                }
            }
            impl<W: Writer> Writable<W>for $T
            {
                fn write_to(&self, writer: &mut W) -> Result<u64, W::Error>
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
        FourCC(val.to_be_bytes())
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
        u32::from_be_bytes(self.0)
    }
}

impl<R: Reader> Readable<R> for FourCC
{
    type Args = ();
    fn read_from(reader: &mut R, (): ()) -> Result<FourCC, R::Error>
    {
        // TODO: Verify ordering
        let bytes = [
            reader.read_u8()?, reader.read_u8()?,
            reader.read_u8()?, reader.read_u8()?,
        ];
        Ok(FourCC(bytes))
    }

    fn fixed_size() -> Option<usize>
    {
        Some(4)
    }
}

impl<W: Writer> Writable<W> for FourCC
{
    fn write_to(&self, writer: &mut W) -> Result<u64, W::Error>
    {
        writer.write_bytes(&self.0)?;
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


impl<R, T> Readable<R> for Option<T>
    where R: Reader,
          T: Readable<R>
{
    type Args = Option<T::Args>;
    fn read_from(reader: &mut R, args: Self::Args) -> Result<Option<T>, R::Error>
    {
        if let Some(args) = args {
            let res = reader.read(args)?;
            Ok(Some(res))
        } else {
            Ok(None)
        }
    }

    fn size(&self) -> Result<usize, R::Error>
    {
        self.as_ref().map(|i| i.size()).unwrap_or(Ok(0))
    }
}

impl<W, T> Writable<W> for Option<T>
    where W: Writer,
          T: Writable<W>
{
    fn write_to(&self, writer: &mut W) -> Result<u64, W::Error>
    {
        if let Some(ref i) = *self {
            i.write_to(writer)
        } else {
            Ok(0)
        }
    }
}

impl<R, T> Readable<R> for Box<T>
    where R: Reader,
          T: Readable<R>
{
    type Args = T::Args;
    fn read_from(reader: &mut R, args: T::Args) -> Result<Box<T>, R::Error>
    {
        Ok(Box::new(reader.read(args)?))
    }

    fn size(&self) -> Result<usize, R::Error>
    {
        <T as Readable<R>>::size(&self)
    }

    fn fixed_size() -> Option<usize>
    {
        T::fixed_size()
    }
}

impl<W, T> Writable<W> for Box<T>
    where W: Writer,
          T: Writable<W>
{
    fn write_to(&self, writer: &mut W) -> Result<u64, W::Error>
    {
        (**self).write_to(writer)
    }
}


impl<R: Reader, T> Readable<R> for PhantomData<T>
{
    type Args = ();
    fn read_from(_reader: &mut R, (): ()) -> Result<Self, R::Error>
    {
        Ok(PhantomData)
    }

    fn fixed_size() -> Option<usize>
    {
        Some(0)
    }
}

impl<W: Writer, T> Writable<W> for PhantomData<T>
{
    fn write_to(&self, _: &mut W) -> Result<u64, W::Error>
    {
        Ok(0)
    }
}

/// Convience alias for a Cow CStr
///
/// It's worth noting that the `Readable` impl for this type _does_ allocate heap memory.
// TODO: It might be worth while to make this not based on ffi::CStr and instead hold a Reader
//       instance. Might be worth profiling both ways later (especially the memory impact)
pub type CStr<'r> = Cow<'r, ffi::CStr>;
impl<'a, R: Reader> Readable<R> for CStr<'a>
{
    type Args = ();
    fn read_from(reader: &mut R, (): ()) -> Result<CStr<'a>, R::Error>
    {
        let mut vec = vec![];
        // XXX A possible optimization would be to use from_bytes_with_nul_unchecked here
        loop {
            let b = reader.read_u8()?;
            if b == 0 {
                break;
            }
            vec.push(b);
        }

        Ok(Cow::Owned(unsafe { ffi::CString::from_vec_unchecked(vec) }))
    }

    fn size(&self) -> Result<usize, R::Error>
    {
        Ok(self.to_bytes_with_nul().len())
    }
}

impl<'a, W: Writer> Writable<W> for CStr<'a>
{
    fn write_to(&self, writer: &mut W) -> Result<u64, W::Error>
    {
        let slice = self.to_bytes_with_nul();
        writer.write_bytes(slice)?;
        Ok(slice.len() as u64)
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
        Cow::Borrowed(ffi::CStr::from_bytes_with_nul(self).unwrap())
    }
}
