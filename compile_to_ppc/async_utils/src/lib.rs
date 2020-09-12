#![cfg_attr(not(test), no_std)]

extern crate alloc;

use futures::task::noop_waker;
use generic_array::{ArrayLength, GenericArray};
use pin_utils::pin_mut;

use alloc::borrow::Borrow;
use core::cell::{Cell, RefCell};
use core::convert::Infallible;
use core::future::Future;
use core::marker::Unpin;
use core::mem::{self, MaybeUninit};
use core::pin::Pin;
use core::ptr;
use core::task::{Context, Poll};

pub mod io;


pub fn poll_until_complete<F: Future>(f: F) -> F::Output
{
    pin_mut!(f);
    let waker = noop_waker();
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
    where F: FnMut(&mut Context<'_>) -> Poll<T>
{
    type Output = T;
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output>
    {
        unsafe { (self.get_unchecked_mut().0)(cx) }
    }
}

pub fn poll_fn<T, F>(f: F) -> PollFn<F>
    where F: FnMut(&mut Context<'_>) -> Poll<T>
{
    PollFn(f)
}

pub fn poll_until<F>(mut f: F) -> impl Future<Output = ()>
    where F: FnMut() -> bool
{
    poll_fn(move |_cx| {
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
        let msg_ptr = poll_fn(|_cx| {
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
    type Output = Infallible;
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

