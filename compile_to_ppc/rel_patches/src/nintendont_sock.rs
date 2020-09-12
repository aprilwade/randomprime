#![allow(unused)]

use alloc::vec::Vec;
use core::cell::RefCell;
use core::convert::Infallible;
use core::future::Future;
use core::marker::PhantomData;
use core::mem::{self, MaybeUninit};
use core::num::NonZeroU32;
use core::pin::Pin;
use core::ptr;
use core::task::{Context, Poll, Waker};

use smallvec::SmallVec;

use async_utils::poll_fn;
use async_utils::io::{AsyncRead, AsyncWrite};

const SO_INADDR_ANY: u32 = 0x00000000;

const SO_PF_INET: i32 = 2;
const SO_F_GETFL: i32 = 3;
const SO_F_SETFL: i32 = 4;
const SO_O_NONBLOCK: u32 = 0x04;

const SO_POLLRDNORM: u16 = 0x000; // Normal data read
const SO_POLLRDBAND: u16 = 0x000; // Priority data read
const SO_POLLPRI: u16 = 0x000; // High priority data read
const SO_POLLWRNORM: u16 = 0x000; // Normal data write
const SO_POLLWRBAND: u16 = 0x001; // Priority data write
const SO_POLLERR: u16 = 0x002; // Error (revents only)
const SO_POLLHUP: u16 = 0x004; // Disconnected (revents only)
const SO_POLLNVAL: u16 = 0x008; // Invalid fd (revents only)
const SO_POLLIN: u16 = SO_POLLRDNORM | SO_POLLRDBAND;
const SO_POLLOUT: u16 = SO_POLLWRNORM;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SOSockAddrIn
{
    len: u8,
    family: u8,
    port: u16,
    addr: u32,
}

#[repr(C)]
struct SOConfig
{
}

#[repr(C)]
struct SOPollFD
{
    fd: i32,
    events: u16,
    revents: u16,
}

pub type OSTime = u64;

macro_rules! decl_so_api {
    ($($fname:ident: fn($($arg_id:ident: $arg_ty:ty),*) $(-> $ret:ty)?,)+) => {
        #[repr(C)]
        #[allow(non_camel_case_types)]
        enum SoApiOffsets
        {
            $($fname,)*
        }
        $(
            #[allow(non_snake_case, unused)]
            unsafe fn $fname($($arg_id: $arg_ty),*) $(-> $ret)?
            {
                type FnPtr = unsafe extern fn($($arg_ty,)*) $(-> $ret)?;
                let f: FnPtr = mem::transmute(0x93006000 + SoApiOffsets::$fname as u32 * 4);
                f($($arg_id,)*)
            }
        )+
    };
}

decl_so_api!{
    SOInit: fn(),
    SOStartup: fn(config: *const SOConfig) -> i32,
    SOCleanup: fn() -> i32,
    SOSocket: fn(af: i32, type_: i32, protocol: i32) -> i32,
    SOClose: fn(s: i32) -> i32,
    SOListen: fn(s: i32, backlog: i32) -> i32,
    SOAccept: fn(s: i32, sockaddr: *mut SOSockAddrIn) -> i32,
    SOBind: fn(s: i32, sock_addr: *const SOSockAddrIn) -> i32,
    SOShutdown: fn(s: i32, how: i32) -> i32,
    SORecvFrom: fn(s: i32, buf: *mut u8, len: u32, flags: i32, sock_from: *mut ()) -> i32,
    SOSendTo: fn(s: i32, buf: *const u8, len: u32, flags: i32, sock_to: *const ()) -> i32,
    SOSetSockOpt: fn(s: i32, level: i32, optname: i32, optval: *const (), optlen: u32),
    // SOFcntl: fn(s: i32, cmd: i32, ...),
    SOFcntl: fn(s: i32, cmd: i32, arg: u32) -> i32,
    SOPoll: fn(fds: *mut SOPollFD, nfds: u32, timeout: OSTime) -> i32,

    avetcp_init: fn(n: Infallible),
    avetcp_term: fn(n: Infallible),

    dns_set_server: fn(n: Infallible),
    dns_clear_server: fn(n: Infallible),
    dns_open_addr: fn(name: *const u8, len: u32) -> i32,
    dns_get_addr: fn(fd: i32, arr: *mut u32) -> i32,
    dns_close: fn(fd: i32) -> i32,

    tcp_create: fn(n: Infallible),
    tcp_bind: fn(n: Infallible),
    tcp_listen: fn(n: Infallible),
    tcp_stat: fn(n: Infallible),
    tcp_getaddr: fn(n: Infallible),
    tcp_connect: fn(s: i32, arr: *const u32, port: u16) -> i32,
    tcp_accept: fn(n: Infallible),
    tcp_send: fn(n: Infallible),
    tcp_receive: fn(n: Infallible),
    tcp_abort: fn(n: Infallible),
    tcp_delete: fn(n: Infallible),
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Error(NonZeroU32);

fn convert_sock_res(i: i32) -> Result<u32, Error>
{
    if i >= 0 {
        Ok(i as u32)
    } else {
        Err(Error(unsafe { NonZeroU32::new_unchecked((-i) as u32) }))
    }
}

macro_rules! define_error {
    ($($id:ident = $e:expr,)*) => {
        impl Error
        {
            $(
                #[allow(unused)]
                pub const $id: Error = Error(unsafe { NonZeroU32::new_unchecked($e) });
            )*
        }

        impl core::fmt::Debug for Error
        {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result
            {
                write!(f, "Error(")?;
                match self.0.get() {
                    $(
                        $e => write!(f, stringify!($id))?,
                    )*
                    i => write!(f, "{}", i)?,
                }
                write!(f, ")")
            }
        }
    };
}

define_error! {
    E2BIG = 1,
    EACCES = 2,
    EADDRINUSE = 3,
    EADDRNOTAVAIL = 4,
    EAFNOSUPPORT = 5, // 5
    EAGAIN = 6,
    EALREADY = 7,
    EBADFD = 8,
    EBADMSG = 9,
    EBUSY = 10, // 10
    ECANCELED = 11,
    ECHILD = 12,
    ECONNABORTED = 13,
    ECONNREFUSED = 14,
    ECONNRESET = 15, // 15
    EDEADLK = 16,
    EDESTADDRREQ = 17,
    EDOM = 18,
    EDQUOT = 19,
    EEXIST = 20, // 20
    EFAULT = 21,
    EFBIG = 22,
    EHOSTUNREACH = 23,
    EIDRM = 24,
    EILSEQ = 25, // 25
    EINPROGRESS = 26,
    EINTR = 27,
    EINVAL = 28,
    EIO = 29,
    EISCONN = 30, // 30
    EISDIR = 31,
    ELOOP = 32,
    EMFILE = 33,
    EMLINK = 34,
    EMSGSIZE = 35, // 35
    EMULTIHOP = 36,
    ENAMETOOLONG = 37,
    ENETDOWN = 38,
    ENETRESET = 39,
    ENETUNREACH = 40, // 40
    ENFILE = 41,
    ENOBUFS = 42,
    ENODATA = 43,
    ENODEV = 44,
    ENOENT = 45, // 45
    ENOEXEC = 46,
    ENOLCK = 47,
    ENOLINK = 48,
    ENOMEM = 49,
    ENOMSG = 50, // 50
    ENOPROTOOPT = 51,
    ENOSPC = 52,
    ENOSR = 53,
    ENOSTR = 54,
    ENOSYS = 55, // 55
    ENOTCONN = 56,
    ENOTDIR = 57,
    ENOTEMPTY = 58,
    ENOTSOCK = 59,
    ENOTSUP = 60, // 60
    ENOTTY = 61,
    ENXIO = 62,
    EOPNOTSUPP = 63,
    EOVERFLOW = 64,
    EPERM = 65, // 65
    EPIPE = 66,
    EPROTO = 67,
    EPROTONOSUPPORT = 68,
    EPROTOTYPE = 69,
    ERANGE = 70, // 70
    EROFS = 71,
    ESPIPE = 72,
    ESRCH = 73,
    ESTALE = 74,
    ETIME = 75, // 75
    ETIMEDOUT = 76,

    RANDOMPRIME = 0xFFFFFFFF,
}

impl async_utils::io::AsyncIoError for Error
{
    fn write_zero_err() -> Self
    {
        Error::RANDOMPRIME
    }
}

pub enum SocketType
{
    Tcp = 1, Udp = 2,
}

struct SocketApiState
{
    poll_fds: Vec<SOPollFD>,
    wakers: Vec<SmallVec<[Waker; 1]>>,
}

pub struct SocketApi(RefCell<SocketApiState>);

impl SocketApi
{
    pub fn global_instance() -> &'static Self
    {
        static mut INSTANCE: Option<SocketApi> = None;
        unsafe {
            if let Some(inst) = &INSTANCE {
                inst
            } else {
                SOInit();
                SOStartup(ptr::null());
                INSTANCE = Some(SocketApi(RefCell::new(SocketApiState {
                    poll_fds: Vec::new(),
                    wakers: Vec::new(),
                })));
                INSTANCE.as_ref().unwrap()
            }
        }
    }

    pub fn poll(&self)
    {
        let SocketApiState { poll_fds, wakers } = &mut *self.0.borrow_mut();
        unsafe {
            // TODO: Check the
            let i = SOPoll(poll_fds.as_mut_ptr(), poll_fds.len() as u32, 0);
            if i == 0 {
                return;
            }
        }
        for (poll_fd, wakers) in poll_fds.iter_mut().zip(wakers.iter_mut()) {
            if poll_fd.revents != 0 {
                poll_fd.fd = -1;
                for waker in wakers {
                    waker.wake_by_ref();
                }
            }
        }
        let mut i = 0;
        wakers.retain(|_| {
            let ret = poll_fds[i].fd != -1;
            i += 1;
            ret
        });
        poll_fds.retain(|poll_fd| poll_fd.fd != -1);
    }

    fn register_poll_fd(&self, fd: u32, waker: &Waker, events: u16)
    {
        let SocketApiState { poll_fds, wakers } = &mut *self.0.borrow_mut();
        let res = poll_fds.iter_mut()
            .zip(wakers.iter_mut())
            .find(|(pfd, _)| pfd.fd == fd as i32);
        if let Some((pfd, wakers)) = res {
            pfd.events |= events;
            for curr_waker in wakers.iter_mut() {
                // Check if this waker is redundant or can replace another waker
                if curr_waker.will_wake(waker) {
                    return
                } else if waker.will_wake(curr_waker) {
                    *curr_waker = waker.clone();
                    return
                }
            }
            wakers.push(waker.clone())
        } else {
            poll_fds.push(SOPollFD {
                fd: fd as i32,
                events,
                revents: 0,
            });
            wakers.push(SmallVec::from_buf([waker.clone()]));
        }
    }

    pub fn tcp_server(&self, port: u16, backlog: u32) -> Result<TcpListener, Error>
    {
        let s;
        unsafe {
            s = convert_sock_res(SOSocket(SO_PF_INET, SocketType::Tcp as i32, 0))?;
            // Make non-blocking
            convert_sock_res(SOFcntl(s as i32, SO_F_SETFL, SO_O_NONBLOCK))?;

            let addr = SOSockAddrIn {
                len: 8,
                family: SO_PF_INET as u8,
                port,
                addr: SO_INADDR_ANY,
            };
            convert_sock_res(SOBind(s as i32, &addr))?;
            convert_sock_res(SOListen(s as i32, backlog as i32))?;
        }
        Ok(TcpListener(s))
    }

    pub fn tcp_connect(&self, addr: SOSockAddrIn) -> impl Future<Output = Result<TcpStream, Error>>
    {
        ConnectFuture::Uninit(addr.port, addr.addr)
    }

}

enum ConnectFuture
{
    Uninit(u16, u32),
    Init(u32),
}

impl Future for ConnectFuture
{
    type Output = Result<TcpStream, Error>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output>
    {
        match *self {
            ConnectFuture::Uninit(port, addr) => {
                unsafe {
                    let s = convert_sock_res(SOSocket(SO_PF_INET, SocketType::Tcp as i32, 0))?;

                    // Make non-blocking
                    convert_sock_res(SOFcntl(s as i32, SO_F_SETFL, SO_O_NONBLOCK))?;

                    let addr = [0, addr];
                    let arr_addr = &addr as *const u32;
                    match convert_sock_res(tcp_connect(s as i32, arr_addr, port)) {
                        Err(Error::EINPROGRESS) => (),

                        Ok(_) => return Poll::Ready(Ok(TcpStream(s))),
                        Err(e) => return Poll::Ready(Err(e)),
                    }

                    // SocketApi::global_instance().register_poll_fd(s, cx.waker(), SO_POLLOUT);
                    *self = ConnectFuture::Init(s);
                    Poll::Pending
                }
            }
            ConnectFuture::Init(s) => {
                unsafe {
                    let mut poll_fd = SOPollFD {
                        fd: s as i32,
                        events: SO_POLLIN,
                        revents: 0,
                    };
                    // Poll again to check for a spurious wakeup
                    if convert_sock_res(SOPoll(&mut poll_fd, 1, 0))? == 0 {
                        Poll::Pending
                    } else {
                        // TODO: Check for a connection error somehow? Zero-length read?
                        Poll::Ready(Ok(TcpStream(s)))
                    }
                }
            },
        }
    }
}



pub struct TcpListener(u32);

impl TcpListener
{
    pub fn accept(&mut self) -> impl Future<Output = Result<(TcpStream, SOSockAddrIn), Error>>
    {
        let s = self.0;
        let mut addr = SOSockAddrIn {
            len: 8,
            family: 0,
            port: 0,
            addr: 0,
        };
        poll_fn(move |cx| {
            match convert_sock_res(unsafe {SOAccept(s as i32, &mut addr) }) {
                Ok(i) => {
                    Poll::Ready(Ok((TcpStream(i), addr)))
                },
                Err(Error::EAGAIN) => {
                    // SocketApi::global_instance().register_poll_fd(s, cx.waker(), SO_POLLIN);
                    Poll::Pending
                },
                Err(e) => Poll::Ready(Err(e)),
            }
        })
    }
}

impl Drop for TcpListener
{
    fn drop(&mut self) {
        unsafe {
            if let Err(e) = convert_sock_res(SOClose(self.0 as i32)) {
                primeapi::dbg!(e);
            }
        }
    }
}

pub struct TcpStream(u32);
pub struct TcpStreamReadHalf<'a>(u32, PhantomData<&'a TcpStream>);
pub struct TcpStreamWriteHalf<'a>(u32, PhantomData<&'a TcpStream>);

impl TcpStream
{
    pub fn split<'a>(&'a mut self) -> (TcpStreamReadHalf<'a>, TcpStreamWriteHalf<'a>)
    {
        (TcpStreamReadHalf(self.0, PhantomData), TcpStreamWriteHalf(self.0, PhantomData))
    }
}

impl Drop for TcpStream
{
    fn drop(&mut self) {
        let res = (|| -> Result<(), Error> {
            unsafe {
                // Revert this socket to a blocking socket so we can close it without fear of
                // truncating any pending output.
                let val = convert_sock_res(SOFcntl(self.0 as i32, SO_F_GETFL, 0))?;
                convert_sock_res((SOFcntl(self.0 as i32, SO_F_SETFL, val & !SO_O_NONBLOCK)))?;
                convert_sock_res(SOClose(self.0 as i32))?;
            }
            Ok(())
        })();
        if let Err(_e) = res {
            primeapi::dbg!(_e);
        }
    }
}

impl<'a> AsyncRead for TcpStreamReadHalf<'a>
{
    type Error = Error;
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut [MaybeUninit<u8>]
    ) -> Poll<Result<usize, Self::Error>>
    {
        let r = unsafe {
            SORecvFrom(
                self.0 as i32,
                buf as *mut _ as *mut u8,
                buf.len() as u32,
                0,
                ptr::null_mut()
            )
        };
        match convert_sock_res(r) {
            Ok(i) => Poll::Ready(Ok(i as usize)),
            Err(Error::EAGAIN) => {
                // SocketApi::global_instance().register_poll_fd(self.0, cx.waker(), SO_POLLIN);
                Poll::Pending
            },
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}

impl AsyncRead for TcpStream
{
    type Error = Error;
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut [MaybeUninit<u8>]
    ) -> Poll<Result<usize, Self::Error>>
    {
        Pin::new(&mut self.split().0).poll_read(cx, buf)
    }
}


impl<'a> AsyncWrite for TcpStreamWriteHalf<'a>
{
    type Error = Error;
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8]
    ) -> Poll<Result<usize, Self::Error>>
    {
        let r = unsafe {
            SOSendTo(
                self.0 as i32,
                buf as *const [u8] as *const u8,
                buf.len() as u32,
                0,
                ptr::null()
            )
        };
        match convert_sock_res(r) {
            Ok(i) => Poll::Ready(Ok(i as usize)),
            Err(Error::EAGAIN) => {
                // SocketApi::global_instance().register_poll_fd(self.0, cx.waker(), SO_POLLOUT);
                Poll::Pending
            },
            Err(e) => Poll::Ready(Err(e)),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Result<(), Self::Error>>
    {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Result<(), Self::Error>>
    {
        // Make the socket non-blocking so the remaining data in the buffer is transmitted in the
        // background
        let res = convert_sock_res(unsafe { SOFcntl(self.0 as i32, SO_F_SETFL, 0) })
            .and_then(|_| convert_sock_res(unsafe { SOClose(self.0 as i32) }))
            .map(|_| ());
        Poll::Ready(res)
    }
}

impl AsyncWrite for TcpStream
{
    type Error = Error;
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8]
    ) -> Poll<Result<usize, Self::Error>>
    {
        Pin::new(&mut self.split().1).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>>
    {
        Pin::new(&mut self.split().1).poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>>
    {
        Pin::new(&mut self.split().1).poll_close(cx)
    }
}
