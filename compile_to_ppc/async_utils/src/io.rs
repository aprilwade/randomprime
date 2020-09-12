
use alloc::vec::Vec;

use core::borrow::{Borrow, BorrowMut};
use core::future::Future;
use core::mem::{self, MaybeUninit};
use core::pin::Pin;
use core::task::{Context, Poll};

use futures::future::Either;
use futures::ready;

use generic_array::{typenum, GenericArray};

use crate::MaybeUninitSliceExt;

pub trait AsyncIoError
{
    fn write_zero_err() -> Self;
}

impl AsyncIoError for ()
{
    fn write_zero_err() -> Self
    {
        ()
    }
}

impl AsyncIoError for futures::never::Never
{
    fn write_zero_err() -> Self
    {
        panic!()
    }
}

pub trait AsyncRead
{
    type Error;
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut [MaybeUninit<u8>]
    ) -> Poll<Result<usize, Self::Error>>;
}

impl<P> AsyncRead for Pin<P>
    where P: core::ops::DerefMut + Unpin,
          P::Target: AsyncRead,
{
    type Error = <P::Target as AsyncRead>::Error;
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [MaybeUninit<u8>])
        -> Poll<Result<usize, Self::Error>>
    {
        self.get_mut().as_mut().poll_read(cx, buf)
    }
}

impl<T: ?Sized + AsyncRead + Unpin> AsyncRead for &mut T {
    type Error = T::Error;
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [MaybeUninit<u8>])
        -> Poll<Result<usize, Self::Error>>
    {
        Pin::new(&mut **self).poll_read(cx, buf)
    }
}

pub trait AsyncBufRead: AsyncRead
{
    fn poll_fill_buf(
        self: Pin<&mut Self>,
        cx: &mut Context
    ) -> Poll<Result<&[u8], Self::Error>>;
    fn consume(self: Pin<&mut Self>, amt: usize);
}

pub trait AsyncReadExt: AsyncRead {
    fn read<'a>(&'a mut self, buf: &'a mut [MaybeUninit<u8>]) -> Read<'a, Self>
    where
        Self: Unpin,
    {
        Read { reader: self, buf }
    }

    fn line_reader<B>(self, buf: B) -> LineReader<Self, B>
    where
        Self: Unpin + Sized,
        B: BorrowMut<[MaybeUninit<u8>]>,
    {
        LineReader::with_buf(buf, self)
    }
}

impl<R: AsyncRead> AsyncReadExt for R { }


/// Future for the [`read`](super::AsyncReadExt::read) method.
#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct Read<'a, R: ?Sized> {
    reader: &'a mut R,
    buf: &'a mut [MaybeUninit<u8>],
}

impl<R: ?Sized + Unpin> Unpin for Read<'_, R> {}

impl<R: AsyncRead + ?Sized + Unpin> Future for Read<'_, R> {
    type Output = Result<usize, R::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = &mut *self;
        Pin::new(&mut this.reader).poll_read(cx, this.buf)
    }
}


pub trait AsyncWrite
{
    type Error;
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8]
    ) -> Poll<Result<usize, Self::Error>>;
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>>;
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>>;
}

impl<P> AsyncWrite for Pin<P>
    where P: core::ops::DerefMut + Unpin,
          P::Target: AsyncWrite,
{
    type Error = <P::Target as AsyncWrite>::Error;
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8])
        -> Poll<Result<usize, Self::Error>>
    {
        self.get_mut().as_mut().poll_write(cx, buf)
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>>
    {
        self.get_mut().as_mut().poll_flush(cx)
    }
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>>
    {
        self.get_mut().as_mut().poll_close(cx)
    }
}

impl<T: ?Sized + AsyncWrite + Unpin> AsyncWrite for &mut T {
    type Error = T::Error;
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8])
        -> Poll<Result<usize, Self::Error>>
    {
        Pin::new(&mut **self).poll_write(cx, buf)
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>>
    {
        Pin::new(&mut **self).poll_flush(cx)
    }
    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>>
    {
        Pin::new(&mut **self).poll_close(cx)
    }
}

pub trait AsyncWriteExt: AsyncWrite
{
    fn write_all<'a>(&'a mut self, buf: &'a [u8]) -> WriteAll<'a, Self>
        where Self::Error: AsyncIoError,
              Self: Unpin,
    {
        WriteAll {
            writer: self,
            buf,
        }
    }

    fn buf_writer<B>(self, buf: B) -> BufWriter<Self, B>
        where Self: Sized,
              Self::Error: AsyncIoError,
              B: BorrowMut<[MaybeUninit<u8>]>,
    {
        BufWriter {
            inner: self,
            buf,
            valid_range: 0..0,
        }
    }

    fn flush(&mut self) -> Flush<'_, Self>
        where Self: Unpin,
    {
        Flush::new(self)
    }

    fn close(&mut self) -> Close<'_, Self>
        where Self: Unpin,
    {
        Close::new(self)
    }

}

impl<W> AsyncWriteExt for W
where
    W: AsyncWrite + ?Sized,
{ }

/// Future for the [`write_all`](super::AsyncWriteExt::write_all) method.
#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct WriteAll<'a, W: ?Sized> {
    writer: &'a mut W,
    buf: &'a [u8],
}

impl<W: ?Sized + Unpin> Unpin for WriteAll<'_, W> {}

impl<W: AsyncWrite + ?Sized + Unpin> Future for WriteAll<'_, W>
    where W::Error: AsyncIoError,
{
    type Output = Result<(), W::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), W::Error>> {
        let this = &mut *self;
        while !this.buf.is_empty() {
            let n = ready!(Pin::new(&mut this.writer).poll_write(cx, this.buf))?;
            {
                let (_, rest) = mem::replace(&mut this.buf, &[]).split_at(n);
                this.buf = rest;
            }
            if n == 0 {
                return Poll::Ready(Err(W::Error::write_zero_err()))
            }
        }

        Poll::Ready(Ok(()))
    }
}

/// A future used to fully close an I/O object.
///
/// Created by the [`close`] function.
///
/// [`close`]: fn.close.html
#[derive(Debug)]
pub struct Close<'a, W: ?Sized + 'a> {
    writer: &'a mut W,
}

impl<W: ?Sized + Unpin> Unpin for Close<'_, W> {}

impl<'a, W: AsyncWrite + ?Sized> Close<'a, W> {
    pub(super) fn new(writer: &'a mut W) -> Close<'a, W> {
        Close { writer }
    }
}

impl<'a, W: AsyncWrite + ?Sized + Unpin> Future for Close<'a, W> {
    type Output = Result<(), W::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut *self.writer).poll_close(cx)
    }
}

/// Future for the [`flush`](super::AsyncWriteExt::flush) method.
#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct Flush<'a, W: ?Sized> {
    writer: &'a mut W,
}

impl<W: ?Sized + Unpin> Unpin for Flush<'_, W> {}

impl<'a, W: AsyncWrite + ?Sized + Unpin> Flush<'a, W> {
    pub(super) fn new(writer: &'a mut W) -> Self {
        Flush { writer }
    }
}

impl<W> Future for Flush<'_, W>
    where W: AsyncWrite + ?Sized + Unpin,
{
    type Output = Result<(), W::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut *self.writer).poll_flush(cx)
    }
}


pub type LineReaderDefaultBuffer = GenericArray<MaybeUninit<u8>, typenum::U512>;
pub struct LineReader<R, B = LineReaderDefaultBuffer>
{
    reader: R,
    buf: B,
    valid_range: (usize, usize),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum LineReaderError<E>
{
    Inner(E),
    MaxLengthExceeded,
}

impl<E> From<E> for LineReaderError<E>
{
    fn from(e: E) -> Self
    {
        LineReaderError::Inner(e)
    }
}

impl<R> LineReader<R>
{
    pub fn new(reader: R) -> Self
    {
        LineReader{
            reader,
            buf: core::iter::repeat(()).map(|()| MaybeUninit::uninit()).collect(),
            valid_range: (0, 0),
        }
    }
}

impl<R, B> LineReader<R, B>
    where B: BorrowMut<[MaybeUninit<u8>]>,
{
    pub fn with_buf(buf: B, reader: R) -> Self
    {
        LineReader{
            reader,
            buf,
            valid_range: (0, 0),
        }
    }

    pub async fn read_line<'s>(&'s mut self) -> Result<&'s [u8], LineReaderError<R::Error>>
        where R: AsyncRead + Unpin,
    {
        let buf = self.buf.borrow_mut();
        loop {
            {
            let filled_buf = &buf[self.valid_range.0..self.valid_range.1];
            let filled_buf = unsafe { filled_buf.assume_init() };
            for (i, b) in filled_buf.iter().enumerate() {
                if *b == b'\n' {
                    let ret = &buf[self.valid_range.0..self.valid_range.0 + i];
                    self.valid_range.0 += i + 1;
                    return Ok(unsafe { ret.assume_init() })
                }
            }
            }

            let data_len = self.valid_range.1 - self.valid_range.0;
            // Copy the data from the end of buf to its front, if needed
            if self.valid_range.0 > 0 {
                if data_len > 0 {
                    unsafe {
                        core::ptr::copy(
                            buf.as_ptr().offset(self.valid_range.0 as isize),
                            buf.as_mut_ptr() as *mut _,
                            data_len,
                        );
                    }
                }
                self.valid_range = (0, data_len);
            }

            if data_len == buf.len() {
                Err(LineReaderError::MaxLengthExceeded)?;
            }
            self.valid_range.1 += self.reader.read(&mut buf[data_len..]).await? as usize;
        }
    }

    pub fn into_reader_and_buf(self) -> (R, OwnedSlice<B>)
    {
        (self.reader, OwnedSlice::new(self.buf, self.valid_range.0..self.valid_range.1))
    }

    pub fn peek_buf(&self) -> &[u8]
    {
        unsafe { self.buf.borrow()[self.valid_range.0..self.valid_range.1].assume_init() }
    }

    /// Returns and consumes the current contents of the buffer
    pub fn get_buf(&mut self) -> &mut [u8]
    {
        let buf = &mut self.buf.borrow_mut()[self.valid_range.0..self.valid_range.1];
        self.valid_range.0 = 0;
        self.valid_range.1 = 0;
        unsafe { buf.assume_init_mut() }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct OwnedSlice<B>(B, usize, usize);

impl<B> OwnedSlice<B>
{
    pub fn new(buf: B, range: core::ops::Range<usize>) -> Self
    {
        OwnedSlice(buf, range.start, range.end)
    }
}

impl<B> core::ops::Deref for OwnedSlice<B>
    where B: Borrow<[u8]>
{
    type Target = [u8];
    fn deref(&self) -> &[u8]
    {
        &self.0.borrow()[self.1..self.2]
    }
}

impl<B> core::ops::DerefMut for OwnedSlice<B>
    where B: BorrowMut<[u8]>
{
    fn deref_mut(&mut self) -> &mut [u8]
    {
        &mut self.0.borrow_mut()[self.1..self.2]
    }
}

impl<B> Borrow<[u8]> for OwnedSlice<B>
    where B: Borrow<[u8]>
{
    fn borrow(&self) -> &[u8]
    {
        &self.0.borrow()[self.1..self.2]
    }
}

impl<B> BorrowMut<[u8]> for OwnedSlice<B>
    where B: BorrowMut<[u8]>
{
    fn borrow_mut(&mut self) -> &mut [u8]
    {
        &mut self.0.borrow_mut()[self.1..self.2]
    }
}

/// XXX Adapted from futures::io
pub struct BufWriter<W, B> {
    inner: W,
    buf: B,
    valid_range: core::ops::Range<usize>,
}

impl<W, B> BufWriter<W, B>
    where W: AsyncWrite,
          W::Error: AsyncIoError,
          B: BorrowMut<[MaybeUninit<u8>]>
{

    fn flush_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), W::Error>>
    {
        let Self { inner, buf, valid_range } = unsafe { self.get_unchecked_mut() };
        let buf = buf.borrow_mut();
        let mut inner = unsafe { Pin::new_unchecked(inner) };

        let mut ret = Ok(());
        while valid_range.start < valid_range.end {
            let buf = unsafe { buf[valid_range.clone()].assume_init() };
            match ready!(inner.as_mut().poll_write(cx, buf)) {
                Ok(0) => {
                    ret = Err(W::Error::write_zero_err());
                    break;
                }
                Ok(n) => valid_range.start += n,
                Err(e) => {
                    ret = Err(e);
                    break;
                }
            }
        }
        *valid_range = 0..0;
        Poll::Ready(ret)
    }

    /// Gets a reference to the underlying writer.
    pub fn get_ref(&self) -> &W
    {
        &self.inner
    }

    /// Gets a mutable reference to the underlying writer.
    ///
    /// It is inadvisable to directly write to the underlying writer.
    pub fn get_mut(&mut self) -> &mut W
    {
        &mut self.inner
    }

   ///// Gets a pinned mutable reference to the underlying writer.
   /////
   ///// It is inadvisable to directly write to the underlying writer.
   //pub fn get_pin_mut(self: Pin<&mut Self>) -> Pin<&mut W> {
   //    self.inner()
   //}

    /// Consumes this `BufWriter`, returning the underlying writer.
    ///
    /// Note that any leftover data in the internal buffer is lost.
    pub fn into_inner(self) -> W
    {
        self.inner
    }

    /// Returns a reference to the internally buffered data.
    pub fn buffer(&self) -> &[u8]
    {
        unsafe { self.buf.borrow()[self.valid_range.clone()].assume_init() }
    }
}

impl<W, B> AsyncWrite for BufWriter<W, B>
    where W: AsyncWrite,
          W::Error: AsyncIoError,
          B: BorrowMut<[MaybeUninit<u8>]>,
{
    type Error = W::Error;
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Self::Error>>
    {
        let this = unsafe { self.get_unchecked_mut() };

        if buf.len() + this.valid_range.end > this.buf.borrow().len() {
            ready!(unsafe { Pin::new_unchecked(&mut *this).as_mut() }.flush_buf(cx))?;
        }
        if buf.len() >= this.buf.borrow().len() {
            unsafe { Pin::new_unchecked(&mut this.inner) }.poll_write(cx, buf)
        } else {
            this.buf.borrow_mut()[this.valid_range.end..this.valid_range.end + buf.len()]
                .copy_from_slice(<[MaybeUninit<u8>]>::from_inited_slice(buf));
            this.valid_range.end += buf.len();
            Poll::Ready(Ok(buf.len()))
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        ready!(self.as_mut().flush_buf(cx))?;
        unsafe { self.map_unchecked_mut(|this| &mut this.inner) }.poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        ready!(self.as_mut().flush_buf(cx))?;
        unsafe { self.map_unchecked_mut(|this| &mut this.inner) }.poll_close(cx)
    }
}

// impl<W: fmt::Debug> fmt::Debug for BufWriter<W> {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         f.debug_struct("BufWriter")
//             .field("writer", &self.inner)
//             .field("buffer", &format_args!("{}/{}", self.buf.len(), self.buf.capacity()))
//             .field("written", &self.written)
//             .finish()
//     }
// }

pub fn copy_buf<R, W>(reader: R, writer: &mut W) -> CopyBuf<'_, R, W>
    where R: AsyncBufRead,
          W: AsyncWrite + Unpin + ?Sized,
{
    CopyBuf {
        reader,
        writer,
        amt: 0,
    }
}

/// Future for the [`copy_buf()`] function.
#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct CopyBuf<'a, R, W: ?Sized> {
    reader: R,
    writer: &'a mut W,
    amt: u64,
}

impl<R: Unpin, W: ?Sized> Unpin for CopyBuf<'_, R, W> {}

impl<R, W: Unpin + ?Sized> CopyBuf<'_, R, W> {
    fn project(self: Pin<&mut Self>) -> (Pin<&mut R>, Pin<&mut W>, &mut u64) {
        unsafe {
            let this = self.get_unchecked_mut();
            (Pin::new_unchecked(&mut this.reader), Pin::new(&mut *this.writer), &mut this.amt)
        }
    }
}

impl<R, W> Future for CopyBuf<'_, R, W>
    where R: AsyncBufRead,
          W: AsyncWrite + Unpin + ?Sized,
          W::Error: AsyncIoError,
{
    type Output = Result<u64, Either<R::Error, W::Error>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let (mut reader, mut writer, amt) = self.project();
        loop {
            let buffer = ready!(reader.as_mut().poll_fill_buf(cx))
                .map_err(Either::Left)?;
            if buffer.is_empty() {
                ready!(writer.as_mut().poll_flush(cx))
                    .map_err(Either::Right)?;
                return Poll::Ready(Ok(*amt));
            }

            let i = ready!(writer.as_mut().poll_write(cx, buffer))
                .map_err(Either::Right)?;
            if i == 0 {
                return Poll::Ready(Err(Either::Right(W::Error::write_zero_err())))
            }
            *amt += i as u64;
            reader.as_mut().consume(i);
        }
    }
}

pub struct Cursor<T>
{
    pos: usize,
    inner: T,
}

impl<T> Cursor<T>
{
    pub fn new(t: T) -> Cursor<T>
    {
        Cursor {
            pos: 0,
            inner: t,
        }
    }
}

impl<T> AsyncRead for Cursor<T>
    where T: Borrow<[u8]>
{
    type Error = futures::never::Never;
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context,
        buf: &mut [MaybeUninit<u8>]
    ) -> Poll<Result<usize, Self::Error>>
    {
        let Self { pos, inner } = unsafe { Pin::get_unchecked_mut(self) };
        let inner = (*inner).borrow();
        let amt = core::cmp::min(inner.len() - *pos, buf.len());
        buf[..amt]
            .copy_from_slice(<[MaybeUninit<u8>]>::from_inited_slice(&inner[*pos..*pos + amt]));
        *pos += amt;
        Poll::Ready(Ok(amt))
    }
}

impl<T> AsyncBufRead for Cursor<T>
    where T: Borrow<[u8]>
{
    fn poll_fill_buf(
        self: Pin<&mut Self>,
        _cx: &mut Context
    ) -> Poll<Result<&[u8], Self::Error>>
    {
        let Self { pos, inner } = unsafe { Pin::get_unchecked_mut(self) };
        Poll::Ready(Ok(&(*inner).borrow()[*pos..]))
    }
    fn consume(self: Pin<&mut Self>, amt: usize)
    {
        let Self { pos, ..} = unsafe { Pin::get_unchecked_mut(self) };
        *pos += amt;
    }
}

impl<T> AsyncWrite for Cursor<T>
    where T: BorrowMut<[MaybeUninit<u8>]>
{
    type Error = futures::never::Never;
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context,
        buf: &[u8]
    ) -> Poll<Result<usize, Self::Error>>
    {
        let Self { pos, inner } = unsafe { Pin::get_unchecked_mut(self) };
        let inner = (*inner).borrow_mut();
        let amt = core::cmp::min(inner.len() - *pos, buf.len());
        inner[*pos..*pos + amt]
            .copy_from_slice(<[MaybeUninit<u8>]>::from_inited_slice(&buf[..amt]));
        *pos += amt;
        Poll::Ready(Ok(amt))
    }
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Result<(), Self::Error>>
    {
        Poll::Ready(Ok(()))
    }
    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Result<(), Self::Error>>
    {
        Poll::Ready(Ok(()))
    }
}


impl AsyncWrite for Vec<u8>
{
    type Error = futures::never::Never;
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context,
        buf: &[u8]
    ) -> Poll<Result<usize, Self::Error>>
    {
        self.extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Result<(), Self::Error>>
    {
        Poll::Ready(Ok(()))
    }
    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Result<(), Self::Error>>
    {
        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod test
{
    use super::*;

    struct DummyAsyncRead<'a>
    {
        bytes: &'a [u8],
        max_bytes_to_write: usize,
        flip: bool,
    }

    impl<'a> AsyncRead for DummyAsyncRead<'a>
    {
        type Error = futures::never::Never;
        fn poll_read(
            mut self: Pin<&mut Self>,
            _cx: &mut Context,
            buf: &mut [MaybeUninit<u8>]
        ) -> Poll<Result<usize, Self::Error>>
        {
            if self.flip {
                self.flip = false;
                return Poll::Pending
            }
            self.flip = true;
            let amt = core::cmp::min(
                self.bytes.len(),
                core::cmp::min(
                    self.max_bytes_to_write,
                    buf.len()
                )
            );
            buf[..amt].copy_from_slice(<[MaybeUninit<u8>]>::from_inited_slice(&self.bytes[..amt]));
            self.bytes = &self.bytes[amt..];
            Poll::Ready(Ok(amt))
        }
    }

    #[test]
    fn test_line_reader()
    {
        let mut lr = LineReader::new(DummyAsyncRead {
            bytes: b"one\ntwo\nthree\r\nfour\n",
            max_bytes_to_write: usize::max_value(),
            flip: false,
        });
        let expected = [&b"one"[..], b"two", b"three\r", b"four"];
        for bytes in &expected {
            assert_eq!(crate::poll_until_complete(lr.read_line()).unwrap(), *bytes);
        }

        let mut lr = LineReader::new(DummyAsyncRead {
            bytes: b"one\ntwo\nthree\r\nfour\n",
            max_bytes_to_write: 5,
            flip: true,
        });
        let expected = [&b"one"[..], b"two", b"three\r", b"four"];
        for bytes in &expected {
            assert_eq!(crate::poll_until_complete(lr.read_line()).unwrap(), *bytes);
        }
    }

    struct DummyAsyncWrite
    {
        dest: Vec<u8>,
        max_bytes_to_write: usize,
        write_flip: bool,
        flush_flip: bool,
        close_flip: bool,
    }

    impl AsyncWrite for DummyAsyncWrite
    {
        type Error = ();

        fn poll_write(
            mut self: Pin<&mut Self>,
            _cx: &mut Context,
            buf: &[u8]
        ) -> Poll<Result<usize, Self::Error>>
        {
            if self.write_flip {
                self.write_flip = false;
                return Poll::Pending;
            }
            self.write_flip = true;
            let amt = core::cmp::min(buf.len(), self.max_bytes_to_write);
            self.dest.extend_from_slice(&buf[..amt]);
            Poll::Ready(Ok(amt))
        }

        fn poll_flush(mut self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Result<(), Self::Error>>
        {
            if self.flush_flip {
                self.flush_flip = false;
                return Poll::Pending;
            }
            self.flush_flip = true;
            Poll::Ready(Ok(()))
        }

        fn poll_close(mut self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Result<(), Self::Error>>
        {
            if self.close_flip {
                self.close_flip = false;
                return Poll::Pending;
            }
            self.close_flip = true;
            Poll::Ready(Ok(()))
        }
    }

    #[test]
    fn test_buf_writer()
    {
        let mut writer = DummyAsyncWrite {
            dest: vec![],
            max_bytes_to_write: usize::max_value(),
            write_flip: true,
            flush_flip: true,
            close_flip: true,
        };
        {
            let mut buf_writer = (&mut writer).buf_writer(vec![MaybeUninit::uninit(); 16]);

            for bytes in &[&b"one"[..], b"two", b"three", b"four", b"five", b"six"] {
                crate::poll_until_complete(buf_writer.write_all(bytes)).unwrap();
            }
            crate::poll_until_complete(buf_writer.close()).unwrap();
        }
        assert_eq!(writer.dest, b"onetwothreefourfivesix");

        let mut writer = DummyAsyncWrite {
            dest: vec![],
            max_bytes_to_write: 3,
            write_flip: true,
            flush_flip: true,
            close_flip: true,
        };
        {
            let mut buf_writer = (&mut writer).buf_writer(vec![MaybeUninit::uninit(); 16]);

            for bytes in &[&b"one"[..], b"two", b"three", b"four", b"five", b"six"] {
                crate::poll_until_complete(buf_writer.write_all(bytes)).unwrap();
            }
            crate::poll_until_complete(buf_writer.close()).unwrap();
        }
        assert_eq!(writer.dest, b"onetwothreefourfivesix");
    }
}
