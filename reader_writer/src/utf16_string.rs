use std::{
    char::{from_u32_unchecked, REPLACEMENT_CHARACTER},
    fmt,
    cmp,
    str::Chars,
};

use crate::{
    reader::{copy, Readable, Reader, ReaderEx},
    writer::{Writable, Writer},
};

#[derive(Clone)]
pub struct Utf16beStr<R>(R);

impl<R: Reader> Utf16beStr<R>
{
    pub fn chars(&self) -> DecodeUtf16<U16beIter<R>>
    {
        decode_utf16(U16beIter(self.0.clone()))
    }
}

impl<R: Reader> Readable<R> for Utf16beStr<R>
{
    type Args = ();
    fn read_from(reader: &mut R, (): ()) -> Result<Self, R::Error>
    {
        let mut start_reader = reader.clone();
        loop {
            if reader.read::<u16>(())? == 0 {
                break
            }
        }
        let read_len = start_reader.len() - reader.len();
        start_reader.truncate_to(read_len)?;
        Ok(Utf16beStr(start_reader))
    }

    fn size(&self) -> Result<usize, R::Error>
    {
        Ok(self.0.len())
    }
}

impl<R, R2> cmp::PartialEq<Utf16beStr<R2>> for Utf16beStr<R>
    where R: Reader,
          R2: Reader,
{
    fn eq(&self, other: &Utf16beStr<R2>) -> bool
    {
        for (c1, c2) in self.chars().zip(other.chars()) {
            match (c1, c2) {
                (Ok(c1), Ok(c2)) if c1 == c2 => (),
                _ => return false,
            }
        }
        true
    }
}

impl<R: Reader> cmp::PartialEq<str> for Utf16beStr<R>
{
    fn eq(&self, other: &str) -> bool
    {
        for (c1, c2) in self.chars().zip(other.chars()) {
            match (c1, c2) {
                (Ok(c1), c2) if c1 == c2 => (),
                _ => return false,
            }
        }
        true
    }
}

impl<R: Reader> fmt::Debug for Utf16beStr<R>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error>
    {
        fmt::Debug::fmt(
            // TODO: an actual error message
            &self.chars().map(|i| i.unwrap_or_else(|_| panic!())).collect::<String>(),
            f
        )
    }
}

impl<R, W> Writable<W>for Utf16beStr<R>
    where R: Reader,
          W: Writer,
          W::Error: From<R::Error>

{
    fn write_to(&self, w: &mut W) -> Result<u64, W::Error>
    {
        copy(&mut self.0.clone(), w)?;
        Ok(self.0.len() as u64)
    }
}

#[derive(Clone, Debug)]
pub struct U16beIter<R>(R);

impl<R: Reader> Iterator for U16beIter<R>
{
    type Item = Result<u16, R::Error>;
    fn next(&mut self) -> Option<Self::Item>
    {
        if self.0.len() > 0 {
            Some(self.0.read(()))
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
pub enum LazyUtf16beStr<R: Reader>
{
    Owned(String),
    Borrowed(Utf16beStr<R>),
}

impl<R> LazyUtf16beStr<R>
    where R: Reader,
{
    pub fn as_mut_string<'s>(&mut self) -> Result<&mut String, R::Error>
    {
        *self = match self {
            LazyUtf16beStr::Owned(s) => return Ok(s),
            LazyUtf16beStr::Borrowed(s) => {
                LazyUtf16beStr::Owned(s.chars().collect::<Result<_, _>>()?)
            }
        };
        self.as_mut_string()
    }

    pub fn into_string<'s>(self) -> Result<String, R::Error>
    {
        match self {
            LazyUtf16beStr::Owned(s) => return Ok(s),
            LazyUtf16beStr::Borrowed(s) => s.chars().collect()
        }
    }

    pub fn chars<'s>(&'s self) -> LazyUtf16beStrChars<'s, R>
    {
        match *self {
            LazyUtf16beStr::Owned(ref s) => LazyUtf16beStrChars::Owned(s.chars()),
            LazyUtf16beStr::Borrowed(ref s) => LazyUtf16beStrChars::Borrowed(s.chars()),
        }
    }
}
impl<R: Reader> cmp::PartialEq<str> for LazyUtf16beStr<R>
{
    fn eq(&self, other: &str) -> bool
    {
        for (c1, c2) in self.chars().zip(other.chars()) {
            match (c1, c2) {
                (Ok(c1), c2) if c1 == c2 => (),
                _ => return false,
            }
        }
        true
    }
}

impl<R: Reader, R2: Reader> cmp::PartialEq<Utf16beStr<R2>> for LazyUtf16beStr<R>
{
    fn eq(&self, other: &Utf16beStr<R2>) -> bool
    {
        for (c1, c2) in self.chars().zip(other.chars()) {
            match (c1, c2) {
                (Ok(c1), Ok(c2)) if c1 == c2 => (),
                _ => return false,
            }
        }
        true
    }
}


impl<R: Reader, R2: Reader> cmp::PartialEq<LazyUtf16beStr<R2>> for LazyUtf16beStr<R>
{
    fn eq(&self, other: &LazyUtf16beStr<R2>) -> bool
    {
        for (c1, c2) in self.chars().zip(other.chars()) {
            match (c1, c2) {
                (Ok(c1), Ok(c2)) if c1 == c2 => (),
                _ => return false,
            }
        }
        true
    }
}


impl<R: Reader> Readable<R> for LazyUtf16beStr<R>
{
    type Args = ();
    fn read_from(reader: &mut R, (): ()) -> Result<Self, R::Error>
    {
        let s = reader.read(())?;
        Ok(LazyUtf16beStr::Borrowed(s))
    }

    fn size(&self) -> Result<usize, R::Error>
    {
        match self {
            LazyUtf16beStr::Owned(s) => Ok(s.chars().map(|c| c.len_utf16()).sum::<usize>() * 2),
            LazyUtf16beStr::Borrowed(s) => s.size(),
        }
    }
}

impl<R, W> Writable<W>for LazyUtf16beStr<R>
    where R: Reader,
          W: Writer,
          W::Error: From<R::Error>,
{
    fn write_to(&self, w: &mut W) -> Result<u64, W::Error>
    {
        match self {
            LazyUtf16beStr::Borrowed(s) => {
                s.write_to(w)
            },
            LazyUtf16beStr::Owned(s) => {
                let mut sum = 0;
                for i in s.encode_utf16() {
                    sum += i.write_to(w)?
                }
                Ok(sum)
            },
        }
    }
}

// #[derive(Clone)]
pub enum LazyUtf16beStrChars<'s, R>
{
    Owned(Chars<'s>),
    Borrowed(DecodeUtf16<U16beIter<R>>),
}

impl<'s, R> Iterator for LazyUtf16beStrChars<'s, R>
    where R: Reader
{
    type Item = Result<char, R::Error>;
    fn next(&mut self) -> Option<Self::Item>
    {
        match *self {
            LazyUtf16beStrChars::Owned(ref mut c) => c.next().map(Ok),
            LazyUtf16beStrChars::Borrowed(ref mut c) => c.next(),
        }
    }
}

impl<R: Reader> From<String> for LazyUtf16beStr<R>
{
    fn from(s: String) -> LazyUtf16beStr<R>
    {
        // Verify null-terminator
        assert!(s.chars().next_back().unwrap() == '\0');
        LazyUtf16beStr::Owned(s)
    }
}

// XXX The following code is shamelessly adapted from std

pub struct DecodeUtf16<I>
{
    iter: I,
    buf: Option<u16>,
}

#[inline]
fn decode_utf16<E,I: IntoIterator<Item = Result<u16 ,E>>>(iter: I) -> DecodeUtf16<I::IntoIter>
{
    DecodeUtf16 { iter: iter.into_iter(), buf: None }
}

impl<E, I: Iterator<Item = Result<u16, E>>> Iterator for DecodeUtf16<I>
{
    type Item = Result<char, E>;

    fn next(&mut self) -> Option<Self::Item>
    {
        let u = match self.buf.take() {
            Some(buf) => buf,
            None => match self.iter.next()? {
                Ok(u) => u,
                Err(e) => return Some(Err(e)),
            }
        };

        if u < 0xD800 || 0xDFFF < u {
            // SAFETY: not a surrogate
            Some(Ok(unsafe { from_u32_unchecked(u as u32) }))
        } else if u >= 0xDC00 {
            // a trailing surrogate
            Some(Ok(REPLACEMENT_CHARACTER))
        } else {
            let u2 = match self.iter.next() {
                Some(Ok(u2)) => u2,
                Some(Err(e)) => return Some(Err(e)),
                // eof
                None => return Some(Ok(REPLACEMENT_CHARACTER)),
            };
            if u2 < 0xDC00 || u2 > 0xDFFF {
                // not a trailing surrogate so we're not a valid
                // surrogate pair, so rewind to redecode u2 next time.
                self.buf = Some(u2);
                return Some(Ok(REPLACEMENT_CHARACTER));
            }

            // all ok, so lets decode it.
            let c = (((u - 0xD800) as u32) << 10 | (u2 - 0xDC00) as u32) + 0x1_0000;
            // SAFETY: we checked that it's a legal unicode value
            Some(Ok(unsafe { from_u32_unchecked(c) }))
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>)
    {
        let (low, high) = self.iter.size_hint();
        // we could be entirely valid surrogates (2 elements per
        // char), or entirely non-surrogates (1 element per char)
        (low / 2, high)
    }
}

