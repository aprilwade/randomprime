#![allow(unused)]

use core::alloc::Layout;
use core::future::Future;
use core::mem::{self, MaybeUninit};
use core::pin::Pin;
use core::ptr;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicPtr, Ordering};
use core::task::{Context, Poll};
use core::slice;

use generic_array::{ArrayLength, GenericArray};

use async_utils::{PollFn, poll_until};
use primeapi::alignment_utils::{Aligned32, Aligned32Slice, Aligned32SliceMut};

static mut MEM2_HEAP: linked_list_allocator::Heap = linked_list_allocator::Heap::empty();
unsafe fn mem2_alloc(size: usize) -> Option<core::ptr::NonNull<u8>>
{
    if MEM2_HEAP.size() == 0 {
        MEM2_HEAP.init(0x931C3000, 0x93200000 - 0x931C3000);
    }
    MEM2_HEAP.allocate_first_fit(
        Layout::from_size_align_unchecked(size + 31 & !31, 32)
    ).ok()
}

unsafe fn mem2_dealloc(ptr: core::ptr::NonNull<u8>, size: usize)
{
    MEM2_HEAP.deallocate(
        ptr,
        Layout::from_size_align_unchecked(size + 31 & !31, 32)
    )
}

pub struct Mem2Buf(*mut [MaybeUninit<u8>]);

impl Mem2Buf
{
    pub async fn allocate(len: usize) -> Mem2Buf
    {
        let ptr = PollFn(move || {
            if let Some(ptr) = unsafe { mem2_alloc(len) } {
                Poll::Ready(ptr)
            } else {
                Poll::Pending
            }
        }).await;
        Mem2Buf(unsafe { slice::from_raw_parts_mut(ptr.as_ptr() as *mut _, len) })
    }

    pub fn allocate_sync(len: usize) -> Option<Mem2Buf>
    {
        if let Some(ptr) = unsafe { mem2_alloc(len) } {
            Some(Mem2Buf(unsafe { slice::from_raw_parts_mut(ptr.as_ptr() as *mut _, len) }))
        } else {
            None
        }
    }

    pub fn as_aligned_slice<'a>(&'a self) -> Aligned32Slice<'a, MaybeUninit<u8>>
    {
        unsafe { Aligned32Slice::from_slice_unchecked(&mut *self.0) }
    }

    pub fn as_aligned_slice_mut<'a>(&'a mut self) -> Aligned32SliceMut<'a, MaybeUninit<u8>>
    {
        unsafe { Aligned32SliceMut::from_slice_unchecked(&mut *self.0) }
    }
}

impl core::ops::Deref for Mem2Buf
{
    type Target = [MaybeUninit<u8>];
    fn deref(&self) -> &Self::Target
    {
        unsafe { &*self.0 }
    }
}

impl core::ops::DerefMut for Mem2Buf
{
    fn deref_mut(&mut self) -> &mut Self::Target
    {
        unsafe { &mut *self.0 }
    }
}


impl Drop for Mem2Buf
{
    fn drop(&mut self)
    {
        unsafe {
            mem2_dealloc(ptr::NonNull::new_unchecked(self.0 as *mut _), (*self.0).len());
        }
    }
}

pub trait ToIoctlvVec
{
    fn to_ioctlv_vec(self) -> IpcIoctlvVec;
}

impl<'a, 'b, T> ToIoctlvVec for &'a Aligned32Slice<'b, T>
{
    fn to_ioctlv_vec(self) -> IpcIoctlvVec
    {
        IpcIoctlvVec {
            ptr: if self.len() > 0 { self.as_ptr() as *mut _ } else { ptr::null_mut() },
            len: (self.len() * mem::size_of::<T>()) as u32,
        }
    }
}

impl<'a, 'b, T> ToIoctlvVec for & 'a mut Aligned32SliceMut<'b, T>
{
    fn to_ioctlv_vec(self) -> IpcIoctlvVec
    {
        IpcIoctlvVec {
            ptr: if self.len() > 0 { self.as_ptr() as *mut _ } else { ptr::null_mut() },
            len: (self.len() * mem::size_of::<T>()) as u32,
        }
    }
}


#[repr(u32)]
enum IpcMessageType
{
    Open = 1,
    Close = 2,
    Read = 3,
    Write = 4,
    Seek = 5,
    Ioctl = 6,
    Ioctlv = 7,
    Response = 8,
}

#[derive(Copy, Clone)]
#[repr(C, align(32))]
struct IpcMessage
{
    cmd: u32,
    result: i32,

    // Union?
    // req_cmd_or_fd: u32,
    fd: u32,

    msg_data: IpcMessageData,

    // XXX ??? At the very least, we need this to occupy 64 bytes so our starlet code has a place
    //         to temporarily store the command type

    _async0: u32,
    _async1: u32,
    _padding: [MaybeUninit<u32>; 5],
    _relaunch: u32,
}

#[derive(Copy, Clone)]
#[repr(C)]
union IpcMessageData
{
    open: IpcOpenRequest,
    close: IpcCloseRequest,
    read: IpcReadOrWriteRequest,
    write: IpcReadOrWriteRequest,
    seek: IpcSeekRequest,
    ioctl: IpcIoctlRequest,
    ioctlv: IpcIoctlvRequest,
    response: IpcResponse,

    raw: [u32; 5],
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
struct IpcOpenRequest
{
    filepath: *const u8,
    mode: u32,
}

#[derive(Copy, Clone, Debug)]
struct IpcCloseRequest;

#[derive(Copy, Clone, Debug)]
#[repr(C)]
struct IpcReadOrWriteRequest
{
    data: *const u8,
    len: u32,
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
struct IpcSeekRequest
{
    where_: i32,
    whence: i32,
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
struct IpcIoctlRequest
{
    ioctl: u32,
    buf_in: *const u8,
    len_in: u32,
    buf_out: *mut u8,
    len_out: u32,
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
struct IpcIoctlvRequest
{
    ioctl: u32,
    argc_in: u32,
    argc_out: u32,
    argv: *mut IpcIoctlvVec,
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct IpcIoctlvVec
{
    ptr: *mut u8,
    len: u32,
}

impl IpcIoctlvVec
{
    pub unsafe fn from_slice_unchecked<T>(s: &mut [T]) -> Self
    {
        IpcIoctlvVec {
            ptr: if s.len() > 0 { s.as_mut_ptr() as *mut u8 } else { ptr::null_mut() },
            len: s.len() as u32,
        }
    }

}


#[derive(Copy, Clone, Debug)]
#[repr(C)]
struct IpcResponse
{
    ret: u32,
}


extern "C"
{
    fn DCFlushRange(start: *const u8, len: u32);
    fn DCInvalidateRange(start: *const u8, len: u32);
}

// NOTE This clears the cached vs uncached bit too
fn virtual_to_real_addr<T>(p: *const T) -> u32
{
    (p as u32) & 0x3FFFFFFF
}

fn real_to_virtual_cached_addr<T>(i: u32) -> *const T
{
    (i | 0x80000000) as *const T
}

fn real_to_virtual_uncached_addr<T>(i: u32) -> *const T
{
    (i | 0xC0000000) as *const T
}

fn uncached_to_cached_addr<T>(p: *const T) -> *const T
{
    (p as u32& 0xBFFFFFFF) as *const T
}

fn cached_to_uncached_addr<T>(p: *const T) -> *const T
{
    (p as u32 | 0xC0000000) as *const T
}


pub fn running_on_dolphin() -> bool
{
    unsafe { core::ptr::read(0xCD000004 as *mut u32) == 0xFFFFFFFF }
}

fn ipc_msg_addr<T>() -> *const T
{
    if !running_on_dolphin() {
        0xD3026900 as *const _
    } else {
        0x80001FFC as *const _
    }
}


// TODO: It'd be nice if allowed multiple ipc requests to be issued simultatiously (ie, we had
//       an array of ipc_msg_addrs instead of just the one)
async unsafe fn ipc_send_msg(msg_ptr: *mut IpcMessage)
{
    let msg_ptr = virtual_to_real_addr(msg_ptr) as *mut IpcMessage;
    poll_until(|| {
        let null = ptr::null_mut();
        let ipc_reg: &AtomicPtr<IpcMessage> = &*(ipc_msg_addr());
        ipc_reg.compare_and_swap(null, msg_ptr, Ordering::Relaxed) == null
    }).await
}

async unsafe fn ipc_wait_msg(msg_ptr: *const IpcMessage)
{
    DCInvalidateRange(msg_ptr as *const _, mem::size_of::<IpcMessage>() as u32);
    poll_until(|| {
        let msg_ptr = cached_to_uncached_addr(msg_ptr);
        ptr::read_volatile(&(*msg_ptr).cmd) == 0xFF
    }).await
}


async unsafe fn ipc_send_and_wait_msg(msg_ptr: *mut IpcMessage)
{
    DCFlushRange(msg_ptr as *const u8, mem::size_of::<IpcMessage>() as u32);
    ipc_send_msg(msg_ptr).await;
    ipc_wait_msg(msg_ptr).await;
}

pub async fn ios_open<'a>(filepath: Aligned32Slice<'a, u8>, mode: u32) -> i32
{
    async fn inner(filepath: &[u8], mode: u32) -> i32
    {
        unsafe {
            DCFlushRange(filepath.as_ptr(), filepath.len() as u32);
        }

        let mut msg: IpcMessage = IpcMessage {
            cmd: IpcMessageType::Open as u32,
            result: 0,
            fd: 0,

            msg_data: IpcMessageData {
                open: IpcOpenRequest {
                    filepath: virtual_to_real_addr(filepath.as_ptr()) as *const _,
                    mode,
                },
            },

            _async0: 0,
            _async1: 0,
            _padding: [MaybeUninit::uninit(); 5],
            _relaunch: 0,
        };

        unsafe {
            ipc_send_and_wait_msg(&mut msg).await;
        }
        msg.result
    }
    inner(filepath.as_ref(), mode).await
}

pub async fn ios_close(fd: u32) -> i32
{
    let mut msg: IpcMessage = IpcMessage {
        cmd: IpcMessageType::Close as u32,
        result: 0,
        fd,

        msg_data: IpcMessageData {
            close: IpcCloseRequest,
        },

        _async0: 0,
        _async1: 0,
        _padding: [MaybeUninit::uninit(); 5],
        _relaunch: 0,
    };

    unsafe {
        ipc_send_and_wait_msg(&mut msg).await;
    }

    msg.result
}

pub async fn ios_ioctl<'i, 'o, I, O>(
    fd: u32, ioctl: u32,
    buf_in: Aligned32Slice<'i, I>,
    mut buf_out: Aligned32SliceMut<'o, O>,
) -> i32
{
    ios_ioctl_raw(
        fd, ioctl,
        if buf_in.len() > 0 { buf_in.as_ptr() as *const u8 } else { ptr::null()},
        (buf_in.len() * mem::size_of::<I>()) as u32,
        if buf_out.len() > 0 { buf_out.as_mut_ptr() as *mut u8 } else { ptr::null_mut() },
        (buf_out.len() * mem::size_of::<O>()) as u32,
    ).await
}

pub async fn ios_ioctl_raw(
    fd: u32, ioctl: u32,
    buf_in: *const u8, len_in: u32,
    buf_out: *mut u8, len_out: u32,
) -> i32
{
    unsafe {
        DCFlushRange(buf_in, len_in);
        DCFlushRange(buf_out, len_out);
    }
    let mut msg: IpcMessage = IpcMessage {
        cmd: IpcMessageType::Ioctl as u32,
        result: 0,
        fd: fd,

        msg_data: IpcMessageData {
            ioctl: IpcIoctlRequest {
                ioctl,
                buf_in: virtual_to_real_addr(buf_in) as *const _,
                len_in,
                buf_out: virtual_to_real_addr(buf_out) as *mut _,
                len_out,
            },
        },

        _async0: 0,
        _async1: 0,
        _padding: [MaybeUninit::uninit(); 5],
        _relaunch: 0,
    };

    unsafe {
        ipc_send_and_wait_msg(&mut msg).await;
    }
    unsafe {
        DCInvalidateRange(buf_out, len_out);
    }

    msg.result
}

pub async fn ios_ioctlv<I, O, N, M>(
    fd: u32, ioctl: u32,
    argv_in: I,
    argv_out: O,
) -> i32
    where I: Into<GenericArray<IpcIoctlvVec, N>>,
          O: Into<GenericArray<IpcIoctlvVec, M>>,
          N: ArrayLength<IpcIoctlvVec>,
          M: ArrayLength<IpcIoctlvVec>,
          N: core::ops::Add<M>,
          generic_array::typenum::Sum<N, M>: ArrayLength<IpcIoctlvVec>,
{
    let argv_in = argv_in.into();
    let argv_out = argv_out.into();

    let mut argv = Aligned32::new(generic_array::sequence::Concat::concat(argv_in, argv_out));
    ios_ioctlv_raw(fd, ioctl, N::U32, M::U32, argv.as_inner_slice_mut()).await
}

pub async fn ios_ioctlv_raw<'a>(
    fd: u32, ioctl: u32,
    argc_in: u32, argc_out: u32,
    mut argv: Aligned32SliceMut<'a, IpcIoctlvVec>,
) -> i32
{
    async fn inner(
        fd: u32, ioctl: u32,
        argc_in: u32, argc_out: u32,
        argv: &mut [IpcIoctlvVec],
    ) -> i32
    {
        debug_assert_eq!(argv.len() as u32, argc_in + argc_out);

        for vec in argv.iter_mut() {
            unsafe {
                DCFlushRange(vec.ptr, vec.len);
            }
            vec.ptr = virtual_to_real_addr(vec.ptr) as *mut _;
        }

        unsafe {
            DCFlushRange(
                argv.as_mut_ptr() as *mut _,
                (argc_in + argc_out) * mem::size_of::<IpcIoctlvVec>() as u32
            );
        }
        let mut msg: IpcMessage = IpcMessage {
            cmd: IpcMessageType::Ioctlv as u32,
            result: 0,
            fd: fd,

            msg_data: IpcMessageData {
                ioctlv: IpcIoctlvRequest {
                    ioctl,
                    argc_in, argc_out,
                    argv: virtual_to_real_addr(argv.as_mut_ptr()) as *mut IpcIoctlvVec,
                },
            },

            _async0: 0,
            _async1: 0,
            _padding: [MaybeUninit::uninit(); 5],
            _relaunch: 0,
        };

        unsafe {
            ipc_send_and_wait_msg(&mut msg).await;
        }
        for vec in argv.iter_mut() {
            vec.ptr = real_to_virtual_cached_addr::<u8>(vec.ptr as u32) as *mut u8;
            unsafe {
                DCInvalidateRange(vec.ptr, vec.len);
            }
        }

        msg.result
    }
    inner(fd, ioctl, argc_in, argc_out, argv.as_mut()).await
}

