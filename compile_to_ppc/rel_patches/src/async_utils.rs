
use pin_utils::pin_mut;

use core::future::Future;
use core::marker::Unpin;
use core::pin::Pin;
use core::ptr;
use core::task::{Context, Poll, Waker, RawWaker, RawWakerVTable};

pub static EMPTY_RAW_WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
        |p| RawWaker::new(p, &EMPTY_RAW_WAKER_VTABLE),
        |_| (),
        |_| (),
        |_| (),
    );

pub fn empty_raw_waker() -> RawWaker
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
impl<F> Unpin for PollFn<F> {}

impl<T, F> core::future::Future for PollFn<F>
    where F: FnMut() -> Poll<T>
{
    type Output = T;
    fn poll(mut self: Pin<&mut Self>, _ctx: &mut Context) -> Poll<Self::Output>
    {
        (&mut self.0)()
    }
}

pub fn poll_until(mut f: impl FnMut() -> bool) -> impl Future<Output = ()>
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

pub fn delay(ticks: u32) -> impl Future<Output = ()>
{
    extern "C" {
        fn OSGetTime() -> u64;
    }

    let finished = ticks as u64 + unsafe { OSGetTime() };
    poll_until(move || unsafe { OSGetTime() } >= finished)
}

pub fn milliseconds_to_ticks(ms: u32) -> u32
{
    const TB_BUS_CLOCK: u32 = 162000000;
    // const TB_CORE_CLOCK: u32 = 486000000;
    const TB_TIMER_CLOCK: u32 = (TB_BUS_CLOCK / 4000);


    ms * TB_TIMER_CLOCK
}


// It really sucks that this has to be specialized to specifically sockets, but without GAT,
// there's no way to express it
pub struct LineReaderSock<'a>
{
    sock: crate::sock_async::TcpStreamRecv<'a>,
    buf: alloc::vec::Vec<u8>,
    valid_range: (usize, usize),
}

impl<'a> LineReaderSock<'a>
    // where F: FnMut(&'a mut [u8]) -> Fut,
    //       Fut: core::future::Future<Output = Result<u32, E>> + 'a,
{
    pub fn new(sock: crate::sock_async::TcpStreamRecv<'a>) -> Self
    {
        LineReaderSock {
            sock,
            buf: alloc::vec::Vec::new(),
            valid_range: (0, 0),
        }
    }
}

impl<'a> LineReaderSock<'a>
    // where F: FnMut(&'a mut [u8]) -> Fut,
    //       Fut: core::future::Future<Output = Result<u32, E>> + 'a,
{
    pub async fn read_line<'s>(&'s mut self) -> crate::sock_async::Result<&'s [u8]>
    {
        // Copy the data from the end of buf to its front, if needed
        if self.valid_range.0 > 0 {
            unsafe {
                core::ptr::copy(
                    self.buf.as_ptr().offset(self.valid_range.0 as isize),
                    self.buf.as_mut_ptr(),
                    self.valid_range.1 - self.valid_range.0,
                );
            }
        }

        let mut data_len = self.valid_range.1 - self.valid_range.0;
        loop {
            if data_len == self.buf.len() {
                // TODO: It might be good to enforce a maximum size
                self.buf.extend(core::iter::repeat(0).take(1024));
            }
            let read = self.sock.read(&mut self.buf[data_len..]).await? as usize;
            for (i, b) in self.buf[data_len..data_len + read].iter().enumerate() {
                if *b == b'\n' {
                    if i == read - 1{
                        self.valid_range = (0, 0);
                    } else {
                        self.valid_range = (data_len + i + 1, data_len + read);
                    }
                    return Ok(&self.buf[..i + 1]);
                }
            }
            data_len += read;
        }
    }
}
