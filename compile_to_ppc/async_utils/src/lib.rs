#![cfg_attr(not(test), no_std)]

extern crate alloc;

use futures::never::Never;
use generic_array::{typenum, ArrayLength, GenericArray};
use pin_utils::pin_mut;

use alloc::borrow::{Borrow, BorrowMut};
use alloc::boxed::Box;
use core::cell::{Cell, RefCell};
use core::future::Future;
use core::marker::Unpin;
use core::mem::{self, MaybeUninit};
use core::pin::Pin;
use core::ptr;
use core::task::{Context, Poll, Waker, RawWaker, RawWakerVTable};

static EMPTY_RAW_WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
        |p| RawWaker::new(p, &EMPTY_RAW_WAKER_VTABLE),
        |_| (),
        |_| (),
        |_| (),
    );

fn empty_raw_waker() -> RawWaker
{
    RawWaker::new(ptr::null(), &EMPTY_RAW_WAKER_VTABLE)
}

pub fn empty_waker() -> Waker
{
    unsafe { Waker::from_raw(empty_raw_waker()) }
}


pub fn poll_until_complete<F: Future>(f: F) -> F::Output
{
    pin_mut!(f);
    let waker = empty_waker();
    let mut ctx = Context::from_waker(&waker);
    loop {
        let f = f.as_mut();
        match f.poll(&mut ctx) {
            Poll::Ready(i) => return i,
            Poll::Pending => (),
        }
    }
}


pub struct PollFn<F>(pub F);

impl<T, F> Future for PollFn<F>
    where F: FnMut() -> Poll<T>
{
    type Output = T;
    fn poll(self: Pin<&mut Self>, _ctx: &mut Context) -> Poll<Self::Output>
    {
        unsafe { (self.get_unchecked_mut().0)() }
    }
}

pub fn poll_until<F>(mut f: F) -> PollFn<impl FnMut() -> Poll<()>>
    where F: FnMut() -> bool
{
    PollFn(move || {
        if f() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    })
}

pub fn stall_once() -> impl Future<Output = ()>
{
    let mut b = false;
    poll_until(move || {
        if b {
            b = true;
            false
        } else {
            true
        }
    })
}

pub trait MaybeUninitSliceExt
{
    type T;
    unsafe fn assume_init(&self) -> &[Self::T];
    unsafe fn assume_init_mut(&mut self) -> &mut [Self::T];

    fn from_inited_slice(this: &[Self::T]) -> &Self;
    fn from_inited_slice_mut(this: &mut [Self::T]) -> &mut Self;
}

impl<T> MaybeUninitSliceExt for [MaybeUninit<T>]
{
    type T = T;
    unsafe fn assume_init(&self) -> &[T]
    {
        mem::transmute(self)
    }
    unsafe fn assume_init_mut(&mut self) -> &mut [T]
    {
        mem::transmute(self)
    }

    fn from_inited_slice(this: &[T]) -> &Self
    {
        unsafe { mem::transmute(this) }
    }

    fn from_inited_slice_mut(this: &mut [T]) -> &mut Self
    {
        unsafe { mem::transmute(this) }
    }
}

pub trait Rebind1Lifetime<'a>: Sized + Future
{
    type Rebound: Sized + Future<Output = Self::Output>;
    unsafe fn rebind(this: &mut Self) -> &mut Self::Rebound
    {
        mem::transmute(this)
    }

    unsafe fn make_static(this: Self::Rebound) -> Self
    {
        let res = mem::transmute_copy(&this);
        mem::forget(this);
        res
    }
}

#[macro_export]
macro_rules! impl_rebind_lifetime_1 {
    ($name:ident) => {
        impl<'a> $crate::Rebind1Lifetime<'a> for $name<'static>
        {
            type Rebound = $name<'a>;
        }
    }
}

impl<'a, T> Rebind1Lifetime<'a> for Pin<Box<dyn Future<Output = T> + 'static>>
{
    type Rebound = Pin<Box<dyn Future<Output = T> + 'a>>;
}

pub struct Lifetime1Rebinder<'a, T>
{
    t: T,
    pd: core::marker::PhantomData<&'a mut &'a ()>,
}

impl<'a, T> Lifetime1Rebinder<'a, T>
    where T: Rebind1Lifetime<'a>
{
    pub fn new(t: T::Rebound) -> Self
    {
        Lifetime1Rebinder {
            t: unsafe { T::make_static(t) },
            pd: core::marker::PhantomData,
        }
    }

    pub fn rebound(&mut self) -> &mut T::Rebound
    {
        unsafe {
            T::rebind(&mut self.t)
        }
    }

    pub fn rebound_pinned(self: Pin<&mut Self>) -> Pin<&mut T::Rebound>
    {
        unsafe {
            self.map_unchecked_mut(|this| T::rebind(&mut this.t))
        }
    }
}

pub trait AsyncRead
{
    type Error;
    type Future: Future<Output = Result<usize, Self::Error>>
               + for<'a> Rebind1Lifetime<'a>;

    fn async_read<'a>(&'a mut self, buf: &'a mut [MaybeUninit<u8>])
        -> Lifetime1Rebinder<'a, Self::Future>;
    fn async_read_inited<'a>(&'a mut self, buf: &'a mut [u8])
        -> Lifetime1Rebinder<'a, Self::Future>
    {
        self.async_read(<[MaybeUninit<_>]>::from_inited_slice_mut(buf))
    }
}


impl<'s, R> AsyncRead for &'s mut R
    where R: AsyncRead
{
    type Error = R::Error;
    type Future = R::Future;

    fn async_read<'a>(&'a mut self, buf: &'a mut [MaybeUninit<u8>])
        -> Lifetime1Rebinder<'a, Self::Future>
    {
        R::async_read(*self, buf)
    }

}

pub async fn async_write_all<W>(mut writer: W, mut buf: &[u8]) -> Result<(), W::Error>
    where W: AsyncWrite
{
    while buf.len() > 0 {
        let fut = writer.async_write(buf);
        pin_mut!(fut);
        let i = fut.rebound_pinned().await?;
        buf = &buf[i..];
    }
    Ok(())
}


pub type BufferedAsyncWriterDefaultBuffer = GenericArray<MaybeUninit<u8>, typenum::U512>;
pub struct BufferedAsyncWriter<W, B = BufferedAsyncWriterDefaultBuffer>
    where W: AsyncWrite,
          B: BorrowMut<[MaybeUninit<u8>]>,
{
    buf: B,
    buf_len: usize,
    writer: W
}

impl<W> BufferedAsyncWriter<W>
    where W: AsyncWrite
{
    pub fn new(w: W) -> BufferedAsyncWriter<W>
    {
        Self::with_buf(core::iter::repeat(()).map(|()| MaybeUninit::uninit()).collect(), w)
    }
}

impl<W, B> BufferedAsyncWriter<W, B>
    where W: AsyncWrite,
          B: BorrowMut<[MaybeUninit<u8>]>,
{
    pub fn with_buf(buf: B, w: W) -> Self
    {
        BufferedAsyncWriter {
            buf,
            buf_len: 0,
            writer: w,
        }
    }

    pub async fn write(&mut self, buf: &[u8]) -> Result<usize, W::Error>
    {
        if buf.len() + self.buf_len > self.buf.borrow().len() {
            self.flush().await?;
        }
        if buf.len() >= self.buf.borrow().len() {
            let fut = self.writer.async_write(buf);
            pin_mut!(fut);
            fut.rebound_pinned().await
        } else {
            let buf = <[MaybeUninit<u8>]>::from_inited_slice(buf);
            self.buf.borrow_mut()[self.buf_len..][..buf.len()].copy_from_slice(buf);
            self.buf_len += buf.len();
            Ok(buf.len())
        }
    }

    pub async fn flush<'a>(&'a mut self) -> Result<(), W::Error>
    {
        let len = self.buf_len;
        if len > 0 {
            self.buf_len = 0;
            async_write_all(&mut self.writer, unsafe { self.buf.borrow()[..len].assume_init() }).await
        } else {
            Ok(())
        }
    }
}

impl<W, B> Drop for BufferedAsyncWriter<W, B>
    where W: AsyncWrite,
          B: BorrowMut<[MaybeUninit<u8>]>,
{
    fn drop(&mut self)
    {
        let _ = poll_until_complete(self.flush());
    }
}


pub trait AsyncWrite
{
    type Error;
    type Future: Future<Output = Result<usize, Self::Error>> + for<'a> Rebind1Lifetime<'a>;

    fn async_write<'a>(&'a mut self, buf: &'a [u8]) -> Lifetime1Rebinder<'a, Self::Future>;
}


impl<'s, W> AsyncWrite for &'s mut W
    where W: AsyncWrite
{
    type Error = W::Error;
    type Future = W::Future;

    fn async_write<'a>(&'a mut self, buf: &'a [u8]) -> Lifetime1Rebinder<'a, Self::Future>
    {
        W::async_write(*self, buf)
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
        where R: AsyncRead,
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
            let fut = self.reader.async_read(&mut buf[data_len..]);
            pin_mut!(fut);
            self.valid_range.1 += fut.rebound_pinned().await? as usize;
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

struct Msg<T: ?Sized>
{
    next: Option<ptr::NonNull<Msg<T>>>,
    data: Option<ptr::NonNull<T>>,
}
pub struct AsyncMsgQueue<T: ?Sized>
{
    head_tail: Cell<Option<(ptr::NonNull<Msg<T>>, ptr::NonNull<Msg<T>>)>>,
}
pub struct MsgRef<T: ?Sized>(ptr::NonNull<Msg<T>>);

impl<T: ?Sized> AsyncMsgQueue<T>
{
    pub fn new() -> Self
    {
        AsyncMsgQueue {
            head_tail: Cell::new(None),
        }
    }

    /// Adds message to the queue and waits until it is handled
    pub async fn sync_push<B: Borrow<T>>(&self, data: B)
    {
        let mut msg = Msg {
            next: None,
            data: ptr::NonNull::new(data.borrow() as *const _ as *mut _),
        };
        let ptr = unsafe { ptr::NonNull::new_unchecked(&mut msg) };
        if let Some((head, tail)) = self.head_tail.take() {
            let tail_msg = unsafe { &mut *tail.as_ptr() };
            tail_msg.next = Some(ptr);
            self.head_tail.set(Some((head, ptr)));
        } else {
            self.head_tail.set(Some((ptr, ptr)));
        }
        poll_until(|| unsafe { ptr.as_ref() }.data.is_none()).await
    }

    fn pop(&self) -> Option<ptr::NonNull<Msg<T>>>
    {
        let (head, tail) = self.head_tail.take()?;
        let next_ptr = unsafe { &mut *head.as_ptr() }.next;

        if let Some(next) = next_ptr {
            self.head_tail.set(Some((next, tail)));
        } else {
            self.head_tail.set(None);
        }

        Some(head)
    }

    /// Dequeues a message; blocks until a message becomes available if the queue is empty
    pub async fn sync_pop(&self) -> MsgRef<T>
    {
        let msg_ptr = PollFn(|| {
            if let Some(msg) = self.pop() {
                Poll::Ready(msg)
            } else {
                Poll::Pending
            }
        }).await;
        MsgRef(msg_ptr)
    }
}

impl<T: ?Sized> core::ops::Deref for MsgRef<T>
{
    type Target = T;
    fn deref(&self) -> &Self::Target
    {
        let msg = unsafe { &mut *self.0.as_ptr() };
        unsafe { &*msg.data.unwrap().as_ptr() }
    }
}

impl<T: ?Sized> Drop for MsgRef<T>
{
    fn drop(&mut self)
    {
        let msg = unsafe { &mut *self.0.as_ptr() };
        msg.data = None;
    }
}

pub struct FutureQueue<F, N: ArrayLength<RefCell<Option<F>>>>
{
    array: GenericArray<RefCell<Option<F>>, N>,
}

pub struct FutureQueuePusher<'a, F, N: ArrayLength<RefCell<Option<F>>>>(&'a FutureQueue<F, N>);
pub struct FutureQueuePoller<'a, F, N: ArrayLength<RefCell<Option<F>>>>(&'a FutureQueue<F, N>);

impl<F, N> FutureQueue<F, N>
    where N: generic_array::ArrayLength<RefCell<Option<F>>>
{
    pub fn new() -> Self
    {
        FutureQueue {
            array: core::iter::repeat(()).map(|()| RefCell::new(None)).collect(),
        }
    }

    pub fn split<'a>(&'a mut self) -> (FutureQueuePoller<'a, F, N>, FutureQueuePusher<'a, F, N>)
    {
        (FutureQueuePoller(self), FutureQueuePusher(self))
    }
}

impl<'a, F, N> FutureQueuePusher<'a, F, N>
    where N: generic_array::ArrayLength<RefCell<Option<F>>>
{
    pub async fn push(&mut self, f: F)
    {
        let mut f = Some(f);
        poll_until(move || {
            let empty_slot = self.0.array.iter()
                .find(|slot| slot.try_borrow().map(|slot| slot.is_none()).unwrap_or(false));
            if let Some(empty_slot) = empty_slot {
                *empty_slot.borrow_mut() = f.take();
                true
            } else {
                false
            }
        }).await
    }
}

impl<'a, F, N> Future for FutureQueuePoller<'a, F, N>
    where N: generic_array::ArrayLength<RefCell<Option<F>>>,
          F: Future<Output = ()> + Unpin,
{
    type Output = Never;
    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output>
    {
        for slot in self.0.array.iter() {
            let mut slot = slot.borrow_mut();
            let finished = if let Some(fut) = &mut *slot {
                Pin::new(fut).poll(ctx).is_ready()
            } else {
                continue
            };

            if finished {
                *slot = None;
            }
        }
        Poll::Pending
    }
}

#[cfg(test)]
mod test
{
    use super::*;

    #[derive(Copy, Clone, Debug)]
    enum Empty { }

    struct DummyAsyncCopy<'a>
    {
        bytes_to_write: &'static [u8],
        max: usize,
        counter: &'a mut usize,
        dst_buf: &'a mut [MaybeUninit<u8>],
    }
    impl<'a> Future for DummyAsyncCopy<'a>
    {
        type Output = Result<usize, Empty>;
        fn poll(mut self: Pin<&mut Self>, _ctx: &mut Context) -> Poll<Self::Output>
        {
            let len = *[self.bytes_to_write.len(), self.dst_buf.len(), self.max].iter()
                .min()
                .unwrap();
            *self.counter += len;
            // DerefMut weirdness...
            let this = &mut *self;
            unsafe {
                core::ptr::copy_nonoverlapping(
                    this.bytes_to_write.as_ptr(),
                    this.dst_buf.as_mut_ptr() as *mut u8,
                    len
                );
            }
            Poll::Ready(Ok(len))
        }
    }
    impl_rebind_lifetime_1!(DummyAsyncCopy);

    struct DummyAsyncReader(usize, usize, &'static [u8]);
    impl AsyncRead for DummyAsyncReader
    {
        type Error = Empty;
        type Future = DummyAsyncCopy<'static>;

        fn async_read<'a>(&'a mut self, buf: &'a mut [MaybeUninit<u8>])
            -> Lifetime1Rebinder<'a, Self::Future>
        {
            Lifetime1Rebinder::new(DummyAsyncCopy {
                bytes_to_write: &self.2[self.0..],
                max: self.1,
                counter: &mut self.0,
                dst_buf: buf,
            })
        }
    }

    #[test]
    fn test_line_reader()
    {
        let reader = DummyAsyncReader(0, usize::max_value(), b"one\ntwo\nthree\r\nfour\n");
        let mut lr = LineReader::new(reader);
        let expected = [&b"one"[..], b"two", b"three\r", b"four"];
        for bytes in &expected {
            assert_eq!(poll_until_complete(lr.read_line()).unwrap(), *bytes);
        }

        let reader = DummyAsyncReader(0, 5, b"one\ntwo\nthree\r\nfour\n");
        let mut lr = LineReader::new(reader);
        let expected = [&b"one"[..], b"two", b"three\r", b"four"];
        for bytes in &expected {
            assert_eq!(poll_until_complete(lr.read_line()).unwrap(), *bytes);
        }
    }

    #[test]
    fn test_msg_queue()
    {
        use futures::future::join;
        let queue = AsyncMsgQueue::new();
        let queue = &queue;

        let make_push_fut = |i| Box::pin(async move {
            queue.sync_push(i).await;
        });
        let f = make_push_fut(0u32);
        let f = join(f, make_push_fut(1u32));
        let f = join(f, make_push_fut(2u32));
        let f = join(f, async {
            assert_eq!(*queue.sync_pop().await, 0);
            assert_eq!(*queue.sync_pop().await, 1);
            assert_eq!(*queue.sync_pop().await, 2);
        });
        poll_until_complete(f);
    }
}

