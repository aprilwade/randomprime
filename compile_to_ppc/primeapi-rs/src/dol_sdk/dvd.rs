use core::cell::UnsafeCell;
use core::fmt;
use core::mem::MaybeUninit;
use core::ptr;

use crate::alignment_utils::Aligned32;

pub type DVDCBCallback = extern "C" fn(result: i32, block: *mut DVDCommandBlock);

#[repr(C)]
#[derive(Debug)]
pub struct DVDCommandBlock
{
    pub next: *mut DVDCommandBlock,
    pub prev: *mut DVDCommandBlock,
    pub command: u32,
    pub state: i32,
    pub offset: u32,
    pub length: u32,
    pub addr: *mut u8,
    pub curr_transfer_state: u32,
    pub transferred_size: u32,

    pub id: *mut u8,// XXX DVDDiskID
    pub callback: DVDCBCallback,
    pub user_data: *mut u8,
}


#[repr(C)]
#[derive(Debug)]
pub struct RawDVDFileInfo
{
    pub cb: DVDCommandBlock,

    pub start_addr: u32,
    pub length: u32,
    pub callback: DVDCallback,
}


pub type DVDCallback = extern "C" fn(result: i32, file_info: *mut RawDVDFileInfo);

extern "C" {
    fn DVDOpen(file_name: *const u8, file_info: *mut RawDVDFileInfo) -> u8;
    fn DVDReadAsyncPrio(file_info: *mut RawDVDFileInfo, addr: *mut u8, length: u32, offset: u32, callback: Option<DVDCallback>, prio: u32) -> u8;
    fn DVDClose(file_info: *mut RawDVDFileInfo) -> u8;
}


pub struct DVDFileInfo(UnsafeCell<RawDVDFileInfo>);

impl fmt::Debug for DVDFileInfo
{
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error>
    {
        let raw_fi = unsafe { ptr::read(self.0.get()) };
        fmt::Debug::fmt(&raw_fi, f)
    }
}

pub struct AsyncDVDReadHandle<'a, F>
{
    state_ptr: ptr::NonNull<i32>,
    phantom: core::marker::PhantomData<(&'a mut F, &'a mut [u8], &'a mut DVDFileInfo)>
}

impl DVDFileInfo
{
    #[inline(always)]
    pub fn new(filename: &[u8]) -> Option<Self>
    {
        let mut fi = MaybeUninit::<RawDVDFileInfo>::uninit();
        unsafe {
            if DVDOpen(filename.as_ptr(), fi.as_mut_ptr()) != 0 {
                Some(DVDFileInfo(UnsafeCell::new(fi.assume_init())))
            } else {
                None
            }
        }
    }

    #[inline(always)]
    pub fn file_length(&self) -> u32
    {
        unsafe { (*self.0.get()).length }
    }

    #[inline(always)]
    pub fn read_async<'a>(
        &'a mut self,
        buf: &'a mut Aligned32<[MaybeUninit<u8>]>,
        offset: u32,
        prio: u32
    ) -> AsyncDVDReadHandle<'a, ()>
    {
        unsafe {
            let state_ptr: *mut i32 = &mut (*self.0.get()).cb.state;
            DVDReadAsyncPrio(
                self.0.get(),
                buf.as_mut_ptr() as *mut u8,
                buf.len() as u32,
                offset,
                None,// callback,
                prio
            );
            AsyncDVDReadHandle {
                state_ptr: ptr::NonNull::new_unchecked(state_ptr),
                phantom: core::marker::PhantomData
            }
        }
    }

    pub fn read_async_with_callback<'a, F>(
        &'a mut self,
        buf: &'a mut Aligned32<[MaybeUninit<u8>]>,
        offset: u32,
        prio: u32,
        cb: &'a mut F
    ) -> AsyncDVDReadHandle<'a, F>
        where F: FnMut(i32)
    {
        extern "C" fn callback<F>(status: i32, fi: *mut RawDVDFileInfo)
            where F: FnMut(i32)
        {
            unsafe {
                let f = &mut *((*fi).cb.user_data as *mut F);
                f(status)
            }
        }

        unsafe {
            (*self.0.get()).cb.user_data = cb as *mut _ as *mut u8;
            let state_ptr: *mut i32 = &mut (*self.0.get()).cb.state;
            DVDReadAsyncPrio(
                self.0.get(),
                buf.as_mut_ptr() as *mut u8,
                buf.len() as u32,
                offset,
                Some(callback::<F>),
                prio
            );
            AsyncDVDReadHandle {
                state_ptr: ptr::NonNull::new_unchecked(state_ptr),
                phantom: core::marker::PhantomData
            }
        }
    }
}

impl Drop for DVDFileInfo
{
    #[inline(always)]
    fn drop(&mut self)
    {
        unsafe {
            DVDClose(self.0.get());
        }
    }
}

impl<'a, F> Drop for AsyncDVDReadHandle<'a, F>
{
    fn drop(&mut self)
    {
        while !self.is_finished() { }
    }
}

impl<'a, F> AsyncDVDReadHandle<'a, F>
{
    #[inline(always)]
    pub fn is_finished(&self) -> bool
    {
        unsafe {
            ptr::read_volatile(self.state_ptr.as_ptr()) == 0x00
        }
    }
}
