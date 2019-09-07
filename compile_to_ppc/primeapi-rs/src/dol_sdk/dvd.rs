use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::ptr;

pub type DVDCBCallback = extern "C" fn(result: i32, block: *mut DVDCommandBlock);

#[repr(C)]
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
    fn DVDReadAsyncPrio(file_info: *mut RawDVDFileInfo, addr: *mut u8, length: u32, offset: u32, callback: DVDCallback, prio: u32) -> u8;
    fn DVDClose(file_info: *mut RawDVDFileInfo) -> u8;
}


pub struct DVDFileInfo(UnsafeCell<RawDVDFileInfo>);

pub struct AsyncDVDReadHandle<'a, 'b>(*mut *mut u8, core::marker::PhantomData<(&'a mut DVDFileInfo, &'b mut [u8])>);

impl DVDFileInfo
{
    pub fn new(filename: &[u8]) -> Self
    {
        let mut fi = MaybeUninit::<RawDVDFileInfo>::uninit();
        unsafe {
            DVDOpen(filename.as_ptr(), fi.as_mut_ptr());
            DVDFileInfo(UnsafeCell::new(fi.assume_init()))
        }
    }

    #[inline(always)]
    pub fn file_length(&self) -> u32
    {
        unsafe { (*self.0.get()).length }
    }

    pub fn read_async<'a, 'b>(&'a mut self, buf: &'b mut [u8], offset: u32, prio: u32)
        -> AsyncDVDReadHandle<'a, 'b>
    {
        extern "C" fn callback(_result: i32, file_info: *mut RawDVDFileInfo)
        {
            unsafe {
                ptr::write_volatile(&mut (*file_info).cb.user_data, 0x00 as *mut u8);
            }
        }
        unsafe {
            ptr::write(&mut (*self.0.get()).cb.user_data, 0x01 as *mut u8);
            DVDReadAsyncPrio(self.0.get(), buf.as_mut_ptr(), buf.len() as u32, offset, callback, prio);
            AsyncDVDReadHandle(&mut (*self.0.get()).cb.user_data, core::marker::PhantomData)
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

impl<'a, 'b> Drop for AsyncDVDReadHandle<'a, 'b>
{
    fn drop(&mut self)
    {
        unsafe {
            while ptr::read_volatile(self.0) == 0x01 as *mut u8 { }
        }
    }
}

