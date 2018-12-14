use std::io;
use std::fmt;
use std::char::{DecodeUtf16, decode_utf16};
use std::str::Chars;

use crate::reader::{Readable, Reader};
use crate::writer::Writable;

#[derive(Clone)]
pub struct Utf16beStr<'a>(Reader<'a>);

impl<'a> Utf16beStr<'a>
{
    pub fn chars(&self) -> DecodeUtf16<U16beIter<'a>>
    {
        decode_utf16(U16beIter(self.0.clone()))
    }
}

impl<'a> Readable<'a> for Utf16beStr<'a>
{
    type Args = ();
    fn read(mut reader: Reader<'a>, (): ()) -> (Self, Reader<'a>)
    {
        let start_reader = reader.clone();
        loop {
            if reader.read::<u16>(()) == 0 {
                break
            }
        }
        let read_len = start_reader.len() - reader.len();
        (Utf16beStr(start_reader.truncated(read_len)), reader)
    }

    fn size(&self) -> usize
    {
        self.0.len()
    }
}

impl<'a> fmt::Debug for Utf16beStr<'a>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error>
    {
        fmt::Debug::fmt(&self.chars().map(|i| i.unwrap()).collect::<String>(), f)
    }
}

impl<'a> Writable for Utf16beStr<'a>
{
    fn write<W: io::Write>(&self, w: &mut W) -> io::Result<()>
    {
        w.write_all(&self.0)
    }
}

#[derive(Clone, Debug)]
pub struct U16beIter<'a>(Reader<'a>);

impl<'a> Iterator for U16beIter<'a>
{
    type Item = u16;
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
pub enum LazyUtf16beStr<'a>
{
    Owned(String),
    Borrowed(Utf16beStr<'a>),
}

impl<'a> LazyUtf16beStr<'a>
{
    pub fn as_mut_string<'s>(&mut self) -> &mut String
    {
        *self = match *self {
            LazyUtf16beStr::Owned(ref mut s) => return s,
            LazyUtf16beStr::Borrowed(ref s) => {
                LazyUtf16beStr::Owned(s.chars().map(|i| i.unwrap()).collect())
            }
        };
        self.as_mut_string()
    }

    pub fn into_string<'s>(self) -> String
    {
        match self {
            LazyUtf16beStr::Owned(s) => return s,
            LazyUtf16beStr::Borrowed(s) => s.chars().map(|i| i.unwrap()).collect(),
        }
    }

    pub fn chars<'s>(&'s self) -> LazyUtf16beStrChars<'a, 's>
    {
        match *self {
            LazyUtf16beStr::Owned(ref s) => LazyUtf16beStrChars::Owned(s.chars()),
            LazyUtf16beStr::Borrowed(ref s) => LazyUtf16beStrChars::Borrowed(s.chars()),
        }
    }
}

impl<'a> Readable<'a> for LazyUtf16beStr<'a>
{
    type Args = ();
    fn read(mut reader: Reader<'a>, (): ()) -> (Self, Reader<'a>)
    {
        let s = reader.read(());
        (LazyUtf16beStr::Borrowed(s), reader)
    }

    fn size(&self) -> usize
    {
        match *self {
            LazyUtf16beStr::Owned(ref s) => s.chars().map(|c| c.len_utf16()).sum::<usize>() * 2,
            LazyUtf16beStr::Borrowed(ref s) => s.size(),
        }
    }
}

impl<'a> Writable for LazyUtf16beStr<'a>
{
    fn write<W: io::Write>(&self, w: &mut W) -> io::Result<()>
    {
        match *self {
            LazyUtf16beStr::Borrowed(ref s) => w.write_all(&s.0),
            LazyUtf16beStr::Owned(ref s) => {
                for i in s.encode_utf16() {
                    i.write(w)?
                }
                Ok(())
            },
        }
    }
}

#[derive(Clone)]
pub enum LazyUtf16beStrChars<'a, 's>
{
    Owned(Chars<'s>),
    Borrowed(DecodeUtf16<U16beIter<'a>>),
}

impl<'a, 's> Iterator for LazyUtf16beStrChars<'a, 's>
{
    type Item = char;
    fn next(&mut self) -> Option<Self::Item>
    {
        match *self {
            LazyUtf16beStrChars::Owned(ref mut c) => c.next().map(|i| i),
            LazyUtf16beStrChars::Borrowed(ref mut c) => c.next().map(|r| r.unwrap_or('\u{fffd}')),
        }
    }
}

impl<'a> From<String> for LazyUtf16beStr<'a>
{
    fn from(s: String) -> LazyUtf16beStr<'a>
    {
        // Verify null-terminator
        assert!(s.chars().next_back().unwrap() == '\0');
        LazyUtf16beStr::Owned(s)
    }
}
