#![allow(unused)]

use crate::ipc_async::{
    ios_open, ios_ioctl, ios_ioctlv, ios_close, running_on_dolphin, IpcIoctlvVec, Mem2Buf,
    ToIoctlvVec,
};

use crate::{delay, milliseconds_to_ticks};
use async_utils::{poll_until_complete, MaybeUninitSliceExt};
use generic_array::{GenericArray, typenum};
use primeapi::alignment_utils::{
    empty_aligned_slice, empty_aligned_slice_mut, Aligned32, Aligned32Slice, Aligned32SliceMut,
    EmptyArray,
};

use core::future::Future;
use core::marker::PhantomData;
use core::mem::{self, MaybeUninit};
use core::num::NonZeroU32;
use core::pin::Pin;
use core::ptr;
use core::slice;
use core::task::{Poll, Context};

macro_rules! decl_consts {
    (@Build { $prev:expr } $id:ident, $($rest:tt)*) => {
        decl_consts!(@Build { $prev } $id = $prev + 1, $($rest)*);
    };
    (@Build { $_prev:expr } $id:ident = $e:expr, $($rest:tt)*) => {
        pub const $id: u32 = $e;
        decl_consts!(@Build { $id } $($rest)*);
    };
    (@Build { $prev:expr }) => {
    };
    ($($tokens:tt)*) => {
        decl_consts!(@Build { 0 } $($tokens)*);
    };
}

// Borrowed from libogc
decl_consts! {
    IOCTL_SO_ACCEPT = 1,
    IOCTL_SO_BIND,
    IOCTL_SO_CLOSE,
    IOCTL_SO_CONNECT,
    IOCTL_SO_FCNTL,
    IOCTL_SO_GETPEERNAME,
    IOCTL_SO_GETSOCKNAME,
    IOCTL_SO_GETSOCKOPT,
    IOCTL_SO_SETSOCKOPT,
    IOCTL_SO_LISTEN,
    IOCTL_SO_POLL,
    IOCTLV_SO_RECVFROM,
    IOCTLV_SO_SENDTO,
    IOCTL_SO_SHUTDOWN,
    IOCTL_SO_SOCKET,
    IOCTL_SO_GETHOSTID,
    IOCTL_SO_GETHOSTBYNAME,
    IOCTL_SO_GETHOSTBYADDR,
    IOCTLV_SO_GETNAMEINFO,
    IOCTL_SO_UNK14,
    IOCTL_SO_INETATON,
    IOCTL_SO_INETPTON,
    IOCTL_SO_INETNTOP,
    IOCTLV_SO_GETADDRINFO,
    IOCTL_SO_SOCKATMARK,
    IOCTLV_SO_UNK1A,
    IOCTLV_SO_UNK1B,
    IOCTLV_SO_GETINTERFACEOPT,
    IOCTLV_SO_SETINTERFACEOPT,
    IOCTL_SO_SETINTERFACE,
    IOCTL_SO_STARTUP,
    IOCTL_SO_ICMPSOCKET = 0x30,
    IOCTLV_SO_ICMPPING,
    IOCTL_SO_ICMPCANCEL,
    IOCTL_SO_ICMPCLOSE,

    IOCTL_NWC24_STARTUP = 0x06,
    IOCTL_NCD_GETLINKSTATUS = 0x07,

    INADDR_ANY = 0,
    INADDR_BROADCAST = 0xffffffff,

    IPPROTO_IP = 0,
    IPPROTO_TCP = 6,
    IPPROTO_UDP = 17,

    SOCK_STREAM = 1,
    SOCK_DGRAM = 2,

    AF_INET = 2,

    SOL_SOCKET = 0xffff,

    SO_SNDBUF = 0x1001, /* send buffer size */
    SO_RCVBUF = 0x1002, /* receive buffer size */
    SO_SNDLOWAT = 0x1003, /* send low-water mark */
    SO_RCVLOWAT = 0x1004, /* receive low-water mark */
    SO_SNDTIMEO = 0x1005, /* send timeout */
    SO_RCVTIMEO = 0x1006, /* receive timeout */
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

#[repr(C)]
#[derive(Debug, Clone)]
pub struct SockAddr
{
    pub len: u8,
    pub family: u8,
    pub port: u16,
    pub name: u32,
    // TODO: How much of this is actually needed?
    pub unused: [u8; 8],
}

pub async fn sock_startup() -> u32
{
    let kd_fd = {
        const KD_FNAME: Aligned32<[u8; 20]> = Aligned32::new(*b"/dev/net/kd/request\0");
        ios_open(KD_FNAME.as_inner_slice(), 0).await
    };

    debug_assert!(kd_fd >= 0);
    let kd_fd = kd_fd as u32;

    // TODO: Spin until this returns 0? I've never seen it return anything else...
    let nwc24_startup_res = {
        let mut output = Aligned32::new([mem::MaybeUninit::<u8>::uninit(); 0x20]);
        ios_ioctl(
            kd_fd,
            IOCTL_NWC24_STARTUP,
            empty_aligned_slice(),
            output.as_inner_slice_mut()
        ).await
    };
    debug_assert_eq!(nwc24_startup_res, 0);

    ios_close(kd_fd).await;

    let so_fd = {
        const SO_FNAME: Aligned32<[u8; 16]> = Aligned32::new(*b"/dev/net/ip/top\0");
        ios_open(SO_FNAME.as_inner_slice(), 0).await
    };
    debug_assert!(so_fd >= 0);
    let so_fd = so_fd as u32;


    let so_startup_res = {
        ios_ioctl(so_fd, IOCTL_SO_STARTUP, empty_aligned_slice(), empty_aligned_slice_mut()).await
    };
    debug_assert_eq!(so_startup_res, 0);

    loop {
        let ip = ios_ioctl(
            so_fd,
            IOCTL_SO_GETHOSTID,
            empty_aligned_slice(),
            empty_aligned_slice_mut()
        ).await;

        if ip != 0 {
            break
        }
        delay(milliseconds_to_ticks(100)).await;
    }

    so_fd
}


pub async fn sock_socket<'a>(
    so_fd: u32,
    domain: u32,
    type_: u32,
    protocol: u32,
) -> Result<u32>
{
    struct SocketParams {
        domain: u32,
        type_: u32,
        protocol: u32,
    }
    let params = Aligned32::new(SocketParams { domain, type_, protocol });
    let r = ios_ioctl(so_fd, IOCTL_SO_SOCKET, params.as_slice(), empty_aligned_slice_mut()).await;
    convert_sock_res(r)
}

pub async fn sock_close<'a>(
    so_fd: u32, socket: u32,
) -> i32
{
    let mut socket = Aligned32::new(socket);
    ios_ioctl(so_fd, IOCTL_SO_CLOSE, socket.as_slice(), empty_aligned_slice_mut()).await
}

pub async fn sock_bind<'a>(
    so_fd: u32, socket: u32,
    sockaddr: SockAddr,
) -> Result<u32>
{
    struct BindParams {
        socket: u32,
        has_name: u32,
        sockaddr: SockAddr,
    }
    let params = Aligned32::new(BindParams {
        socket,
        has_name: 1,
        sockaddr
    });
    let r = ios_ioctl(so_fd, IOCTL_SO_BIND, params.as_slice(), empty_aligned_slice_mut()).await;
    convert_sock_res(r)
}


pub async fn sock_accept<'a>(
    so_fd: u32, socket: u32,
    sockaddr: &mut Aligned32<mem::MaybeUninit<SockAddr>>,
) -> Result<u32>
{
    let mut socket = Aligned32::new(socket);
    unsafe {
        ptr::write((&mut (*sockaddr.as_mut_ptr()).len), 8);
        ptr::write((&mut (*sockaddr.as_mut_ptr()).family), AF_INET as u8);
    }

    let r = ios_ioctl(so_fd, IOCTL_SO_ACCEPT, socket.as_slice(), sockaddr.as_slice_mut()).await;
    convert_sock_res(r)
}

pub async fn sock_connect<'a>(
    so_fd: u32, socket: u32,
    sockaddr: SockAddr,
) -> Result<u32>
{
    struct ConnectParams
    {
        socket: u32,
        sockaddr: SockAddr,
    }
    let params = Aligned32::new(ConnectParams {
        socket,
        sockaddr: SockAddr { len: 8, ..sockaddr },
    });
    convert_sock_res(ios_ioctl(
        so_fd,
        IOCTL_SO_CONNECT,
        params.as_slice(),
        empty_aligned_slice_mut()
    ).await)
}

pub async fn sock_listen<'a>(
    so_fd: u32, socket: u32,
    backlog: u32
) -> Result<u32>
{
    struct ListenParams {
        socket: u32,
        backlog: u32
    }
    let params = Aligned32::new(ListenParams {
        socket,
        backlog
    });
    convert_sock_res(ios_ioctl(
        so_fd,
        IOCTL_SO_LISTEN,
        params.as_slice(),
        empty_aligned_slice_mut()
    ).await)
}

pub async fn sock_sendto<'a>(
    so_fd: u32, socket: u32,
    buf: Aligned32Slice<'a, u8>,
    flags: u32,
    sockaddr: Option<SockAddr>,
) -> Result<u32>
{
    #[repr(C)]
    struct SendToParams {
        socket: u32,
        flags: u32,
        has_destaddr: u32,
        sockaddr: mem::MaybeUninit<SockAddr>,
    }

    let sendto_params = Aligned32::new(SendToParams {
        socket,
        flags,
        has_destaddr: sockaddr.is_some() as u32,
        sockaddr: if let Some(sockaddr) = sockaddr {
                mem::MaybeUninit::new(sockaddr)
            } else {
                mem::MaybeUninit::uninit()
            },
    });
    convert_sock_res(ios_ioctlv(
        so_fd, IOCTLV_SO_SENDTO,
        [buf.to_ioctlv_vec(), sendto_params.as_slice().to_ioctlv_vec()],
        EmptyArray,
    ).await)
}

pub async fn sock_recvfrom(
    so_fd: u32, socket: u32,
    buf: &mut [MaybeUninit<u8>],
    flags: u32,
    sockaddr: Option<&mut Aligned32<mem::MaybeUninit<SockAddr>>> ,
) -> Result<u32>
{
    struct SendToParams {
        socket: u32,
        flags: u32,
    }
    let input_params = Aligned32::new(SendToParams { socket, flags });

    let mut sockaddr_slice = if let Some(sockaddr) = sockaddr {
        unsafe {
            ptr::write(sockaddr.as_mut_ptr() as *mut u8, 8u8);
        }
        sockaddr.as_slice_mut()
    } else {
        Aligned32SliceMut::empty()
    };
    let (mem2_buf, buf_ioctlv_vec) = if running_on_dolphin() {
        // On dolphin we don't actually need any particular alignment
        (None, unsafe { IpcIoctlvVec::from_slice_unchecked(buf) })
    } else {
        // On Nintendont the recv buffer must be in MEM2 otherwise we risk truncation
        let mem2_buf = Mem2Buf::allocate(buf.len()).await;
        let buf_ioctlv_vec = mem2_buf.as_aligned_slice().to_ioctlv_vec();
        (Some(mem2_buf), buf_ioctlv_vec)
    };
    let r = ios_ioctlv(
        so_fd, IOCTLV_SO_RECVFROM,
        [input_params.as_slice().to_ioctlv_vec(),],
        [buf_ioctlv_vec, sockaddr_slice.to_ioctlv_vec()],
    ).await;
    if r > 0 {
        if let Some(mem2_buf) = mem2_buf {
            unsafe {
                ptr::copy_nonoverlapping(
                    mem2_buf.as_ptr(),
                    buf.as_mut_ptr() as *mut _,
                    r as usize
                );
            }
        }
    }
    convert_sock_res(r)
}

pub async fn sock_setsockopt<'a, T>(
    so_fd: u32, socket: u32,
    level: u32, optname: u32,
    optval: &[T],
) -> Result<u32>
{
    struct SetSockOptParams {
        socket: u32,
        level: u32,
        optname: u32,
        optlen: u32,
        optval: [u8; 0x20],
    }

    let optlen = mem::size_of::<T>() * optval.len();
    if optlen > 0x20 {
        Err(Error::EINVAL)?;
    }
    let mut optval_ = [0u8; 0x20];
    unsafe {
        ptr::copy_nonoverlapping(optval.as_ptr() as *const u8, optval_.as_mut_ptr(), optlen);
    }
    let params = Aligned32::new(SetSockOptParams {
        socket,
        level,
        optname,
        optlen: optlen as u32,
        optval: optval_,
    });

    convert_sock_res(ios_ioctl(
        so_fd, IOCTL_SO_SETSOCKOPT,
        params.as_slice(),
        empty_aligned_slice_mut(),
    ).await)
}

pub type Result<T, E = Error> = core::result::Result<T, E>;

#[derive(Debug)]
pub struct SockSystem(u32);

#[derive(Debug)]
pub struct TcpListener
{
    so_fd: u32,
    socket: u32,
}

#[derive(Debug)]
pub struct TcpStream
{
    so_fd: u32,
    socket: u32,
}

impl SockSystem
{
    pub async fn new() -> SockSystem
    {
        SockSystem(sock_startup().await)
    }

    pub async fn tcp_listen(&self, addr: &SockAddr, backlog: u32) -> Result<TcpListener>
    {
        let socket = sock_socket(self.0, AF_INET, SOCK_STREAM, IPPROTO_IP).await?;

        sock_bind(self.0, socket, addr.clone()).await?;
        sock_listen(self.0, socket, backlog).await?;

        Ok(TcpListener { so_fd: self.0, socket })
    }

    pub async fn tcp_connect(&self, addr: &SockAddr) -> Result<TcpStream>
    {
        let socket = sock_socket(self.0, AF_INET, SOCK_STREAM, IPPROTO_IP).await?;

        sock_setsockopt(self.0, socket, SOL_SOCKET, SO_RCVBUF, &[32768u32]).await?;

        sock_connect(self.0, socket, addr.clone()).await?;
        Ok(TcpStream { so_fd: self.0, socket })
    }
}

impl TcpListener
{
    pub async fn accept(&mut self) -> Result<TcpStream>
    {
        let mut sockaddr = Aligned32::new(mem::MaybeUninit::uninit());
        let client_socket = sock_accept(self.so_fd, self.socket, &mut sockaddr).await?;
        Ok(TcpStream {
            so_fd: self.so_fd,
            socket: client_socket,
        })
    }
}

// TODO: Add a method to set the timeout (both recv and send)
impl TcpStream
{
    pub async fn send<'a>(&mut self, buf: Aligned32Slice<'a, u8>) -> Result<u32>
    {
        let mut send = self.split().0;
        send.send(buf).await
    }

    pub async fn recv(&mut self, buf: &mut [MaybeUninit<u8>]) -> Result<u32>
    {
        let mut recv = self.split().1;
        recv.recv(buf).await
    }

    pub async fn send_all(&mut self, buf: &[u8]) -> Result<()>
    {
        let mut send = self.split().0;
        send.send_all(buf).await
    }

    pub fn split<'a>(&'a mut self) -> (TcpStreamSend<'a>, TcpStreamRecv<'a>)
    {
        let send = TcpStreamSend {
            so_fd: self.so_fd,
            socket: self.socket,
            phantom: PhantomData,
        };
        let recv = TcpStreamRecv {
            so_fd: self.so_fd,
            socket: self.socket,
            phantom: PhantomData,
        };
        (send, recv)
    }
}

impl Drop for TcpListener
{
    fn drop(&mut self)
    {
        poll_until_complete(sock_close(self.so_fd, self.socket));
    }
}

impl Drop for TcpStream
{
    fn drop(&mut self)
    {
        poll_until_complete(sock_close(self.so_fd, self.socket));
    }
}

pub struct TcpStreamSend<'a>
{
    so_fd: u32,
    socket: u32,
    phantom: PhantomData<&'a mut TcpStream>,
}

async fn sock_send_unaligned(so_fd: u32, socket: u32, buf: &[u8]) -> Result<u32>
{
    // TODO: This dance isn't necessary on dolphin
    let (unaligned, aligned) = Aligned32Slice::split_unaligned_prefix(buf);
    let i = {
        let mut tmp_buf = Aligned32::new([MaybeUninit::uninit(); 32]);
        tmp_buf[..unaligned.len()].copy_from_slice(<[MaybeUninit<u8>]>::from_inited_slice(unaligned));

        let tmp_buf = tmp_buf.as_inner_slice().truncate_to_len(unaligned.len());
        sock_sendto(so_fd, socket, unsafe { tmp_buf.assume_init() }, 0, None).await?
    };
    if (i as usize) < unaligned.len() {
        Ok(i)
    } else {
        Ok(sock_sendto(so_fd, socket, aligned, 0, None).await? + i)
    }
}

async fn sock_send_unaligned_usize(so_fd: u32, socket: u32, buf: &[u8]) -> Result<usize>
{
    sock_send_unaligned(so_fd, socket, buf).await.map(|i| i as usize)
}


impl<'a> TcpStreamSend<'a>
{
    pub async fn send<'b>(&mut self, buf: Aligned32Slice<'b, u8>) -> Result<u32>
    {
        sock_sendto(self.so_fd, self.socket, buf, 0, None).await
    }

    pub async fn send_unaligned<'b>(&'a mut self, buf: &'b [u8]) -> Result<u32>
    {
        sock_send_unaligned(self.so_fd, self.socket, buf).await
    }

    fn send_unaligned_<'b>(&'b mut self, buf: &'b [u8]) -> TcpStreamSendWrite_<'b>
    {
        sock_send_unaligned_usize(self.so_fd, self.socket, buf)
    }

    pub async fn send_all(&mut self, buf: &[u8]) -> Result<()>
    {
        // TODO: MaybeUninit
        let mut tmp_buf = Aligned32::new(GenericArray::<u8, typenum::U4096>::default());
        let mut bytes_written = 0;
        while bytes_written < buf.len() {
            // TODO: If the remainder of tmp_buf starts at a 32-byte aligned address...
            let send_len = core::cmp::min(buf.len() - bytes_written, tmp_buf.len());
            tmp_buf[..send_len].copy_from_slice(&buf[bytes_written..bytes_written + send_len]);
            let mut tmp_buf = tmp_buf.as_inner_slice().truncate_to_len(send_len);
            bytes_written += self.send(tmp_buf).await? as usize;
        }
        Ok(())
    }

}

type TcpStreamSendWrite_<'a> = impl Future<Output = Result<usize>> + 'a;
pub struct TcpStreamSendWrite<'a>(TcpStreamSendWrite_<'a>);
impl<'a> Future for TcpStreamSendWrite<'a>
{
    type Output = <TcpStreamSendWrite_<'a> as Future>::Output;
    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output>
    {
        unsafe { self.map_unchecked_mut(|this| &mut this.0) }.poll(ctx)
    }
}
async_utils::impl_rebind_lifetime_1!(TcpStreamSendWrite);

impl<'a> async_utils::AsyncWrite for TcpStreamSend<'a>
{
    type Error = Error;
    type Future = TcpStreamSendWrite<'static>;

    fn async_write<'b>(&'b mut self, buf: &'b [u8])
        -> async_utils::Lifetime1Rebinder<'b, Self::Future>
    {
        async_utils::Lifetime1Rebinder::new(TcpStreamSendWrite(self.send_unaligned_(buf)))
    }
}

pub struct TcpStreamRecv<'a>
{
    so_fd: u32,
    socket: u32,
    phantom: PhantomData<&'a mut TcpStream>,
}

impl<'a> TcpStreamRecv<'a>
{
    pub async fn recv(&mut self, buf: &mut [MaybeUninit<u8>]) -> Result<u32>
    {
        sock_recvfrom(self.so_fd, self.socket, buf, 0, None).await
    }

    fn read_usize_<'b>(&'b mut self, buf: &'b mut [MaybeUninit<u8>]) -> TcpStreamRecvRead_<'b>
    {
        // Compiler bug workaraound; apparently combinators won't work
        pub async fn sock_recvfrom_usize(
            so_fd: u32, socket: u32,
            buf: &mut [MaybeUninit<u8>],
            flags: u32,
            sockaddr: Option<&mut Aligned32<mem::MaybeUninit<SockAddr>>> ,
        ) -> Result<usize>
        {
            sock_recvfrom(so_fd, socket, buf, 0, None).await.map(|i| i as usize)
        }
        sock_recvfrom_usize(self.so_fd, self.socket, buf, 0, None)
    }
}

impl<'a> async_utils::AsyncRead for TcpStreamRecv<'a>
{
    type Error = Error;
    type Future = TcpStreamRecvRead<'static>;

    fn async_read<'s>(&'s mut self, buf: &'s mut [MaybeUninit<u8>])
        -> async_utils::Lifetime1Rebinder<'s, Self::Future>
    {
        async_utils::Lifetime1Rebinder::new(TcpStreamRecvRead(self.read_usize_(buf)))
    }
}

type TcpStreamRecvRead_<'a> = impl Future<Output = Result<usize>> + 'a;
// XXX Temporary workaround for a compiler bug
// (https://github.com/rust-lang/rust/issues/63677)
pub struct TcpStreamRecvRead<'a>(TcpStreamRecvRead_<'a>);
impl<'a> Future for TcpStreamRecvRead<'a>
{
    type Output = <TcpStreamRecvRead_<'a> as Future>::Output;
    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output>
    {
        unsafe { self.map_unchecked_mut(|this| &mut this.0) }.poll(ctx)
    }
}
async_utils::impl_rebind_lifetime_1!(TcpStreamRecvRead);
