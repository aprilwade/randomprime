/// IO utilities and combinators adapted from futures-rs

use core::future::Future;
use core::mem::{self, MaybeUninit};
use core::pin::Pin;
use core::task::{Context, Poll};


macro_rules! ready {
    ($e:expr) => {
        match $e {
            core::task::Poll::Ready(e) => e,
            core::task::Poll::Pending => return core::task::Poll::Pending,
        }
    }
}

pub trait AsyncIoError
{
    fn write_zero_err() -> Self;
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

impl<'a, R: AsyncRead + ?Sized + Unpin> Read<'a, R> {
    pub(super) fn new(reader: &'a mut R, buf: &'a mut [MaybeUninit<u8>]) -> Self {
        Read { reader, buf }
    }
}

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
    {
        WriteAll {
            writer: self,
            buf,
        }
    }

    fn close(&mut self) -> Close<'_, Self>
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

impl<'a, W: AsyncWrite + ?Sized + Unpin> WriteAll<'a, W> {
    pub(super) fn new(writer: &'a mut W, buf: &'a [u8]) -> Self {
        WriteAll { writer, buf }
    }
}


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
