#![allow(unused)]
// https://wiibrew.org/wiki/Ipc.c

use core::mem;
use core::ptr;
use core::sync::atomic::{AtomicPtr, Ordering};


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

    // XXX ???

    _async0: u32,
    _async1: u32,
    _padding: [u32; 5],
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
    argv: *mut u8,
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


pub(crate) fn running_on_dolphin() -> bool
{
    unsafe { ipc_read_reg(1) == 0xFFFFFFFF }
}

// #[inline(always)]
// unsafe fn ipc_write_reg(i: usize, val: u32)
// {
//     let ipc_base = 0xCD000000 as *mut u32;
//     core::ptr::write_volatile(ipc_base.add(i), val);
// }

#[inline(always)]
unsafe fn ipc_read_reg(i: usize) -> u32
{
    let ipc_base = 0xCD000000 as *mut u32;
    core::ptr::read_volatile(ipc_base.add(i))
}

// #[inline(always)]
// unsafe fn ipc_ack_irq()
// {
//     core::ptr::write_volatile(0xCD000030 as *mut u32, 0x40000000);
// }

// #[inline(always)]
// unsafe fn ipc_bell(w: u32)
// {
//     ipc_write_reg(1, (ipc_read_reg(1) & 0x30) | w)
// }

// unsafe fn ipc_send(msg: *mut IpcMessage)
// {
//     crate::printf(b"ipc_send(%p)\n\0".as_ptr(), msg as u32);

//     crate::printf(b"ipc_read_ref(3) = %08x\n\0".as_ptr(), ipc_read_reg(3));
//     ipc_write_reg(3, 0x30);
//     crate::printf(b"ipc_read_ref(3) = %08x\n\0".as_ptr(), ipc_read_reg(3));

//     ipc_write_reg(0, virtual_to_real_addr(msg));
//     crate::printf(b"ipc_read_ref(1) = %08x\n\0".as_ptr(), ipc_read_reg(1));
//     ipc_bell(1);

//     crate::printf(b"waiting for ack...\n\0".as_ptr());
//     let mut i = 0;
//     while ipc_read_reg(1) & 0x22 != 0x22 {
//         i += 1;
//         if i == 1000000 {
//             i = 0;
//             crate::printf(b"ipc_read_ref(0) = %08x\n\0".as_ptr(), ipc_read_reg(0));
//             crate::printf(b"ipc_read_ref(1) = %08x\n\0".as_ptr(), ipc_read_reg(1));
//             crate::printf(b"ipc_read_ref(2) = %08x\n\0".as_ptr(), ipc_read_reg(2));
//             crate::printf(b"ipc_read_ref(3) = %08x\n\0".as_ptr(), ipc_read_reg(3));
//         }
//     }
//     ipc_bell(2);
//     ipc_ack_irq();
// }

// unsafe fn ipc_receive() -> bool
// {
//     if ipc_read_reg(1) & 0x14 == 0x14 {
//         let reply_ptr = ipc_read_reg(2);
//         if reply_ptr == 0 {
//             // XXX ??
//         } else {
//             let reply_ptr: *const IpcMessage = real_to_virtual_addr(reply_ptr);
//             DCInvalidateRange(reply_ptr as *const u8, core::mem::size_of::<IpcMessage>() as u32);
//             ipc_bell(4);
//             ipc_bell(8);
//         }

//         true
//     } else {
//         false
//     }
// }

// unsafe fn ipc_wait()
// {
//     crate::printf(b"starting ipc_wait...\n\0".as_ptr());
//     while !ipc_receive() { }
// }

unsafe fn ipc_send_msg(msg_ptr: *mut IpcMessage)
{
    let null = ptr::null_mut();
    let ipc_reg: &AtomicPtr<IpcMessage> = &*(0xD3026900 as *const _);
    let msg_ptr = virtual_to_real_addr(msg_ptr) as *mut IpcMessage;
    while ipc_reg.compare_and_swap(null, msg_ptr, Ordering::Relaxed) != null { }
}

unsafe fn ipc_wait_msg(msg_ptr: *const IpcMessage)
{
    let msg_ptr = cached_to_uncached_addr(msg_ptr);
    while ptr::read_volatile(&(*msg_ptr).cmd) != IpcMessageType::Response as u32 { }
}

#[repr(C, align(32))]
struct AlignedCStr([u8; 64]);

pub(crate) fn ios_open_sync(filepath: &[u8], mode: u32) -> i32
{
    let mut aligned_filepath = AlignedCStr([0; 64]);
    aligned_filepath.0[..filepath.len()].copy_from_slice(filepath);
    unsafe {
        DCFlushRange(aligned_filepath.0.as_ptr(), filepath.len() as u32);
        crate::printf(b"ios_open filepath: %s\n\0".as_ptr(), aligned_filepath.0.as_ptr());
    }


    let mut msg: IpcMessage = IpcMessage {
        cmd: IpcMessageType::Open as u32,
        result: 0,
        fd: 0,

        msg_data: IpcMessageData {
            open: IpcOpenRequest {
                filepath: virtual_to_real_addr(aligned_filepath.0.as_ptr()) as *const _,
                mode,
            },
        },

        _async0: 0,
        _async1: 0,
        _padding: [0; 5],
        _relaunch: 0,
    };

    unsafe {
        DCFlushRange(&msg as *const _ as *const u8, core::mem::size_of::<IpcMessage>() as u32);
        let msg_ptr = &mut msg as *mut _;
        crate::printf(b"Sending msg_ptr: %p\n\0".as_ptr(), msg_ptr);
        ipc_send_msg(msg_ptr);
        ipc_wait_msg(msg_ptr);
    }
    unsafe {
        crate::printf(b"msg_ptr.cmd: %d\n\0".as_ptr(), msg.cmd);
    }

    msg.result
}
