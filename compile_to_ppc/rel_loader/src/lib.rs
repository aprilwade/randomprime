#![no_std]

extern crate alloc as alloc_;
// To pickup alloc/panic impls
extern crate primeapi;

use alloc_::alloc::{alloc, Layout};
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicBool, Ordering};

mod ffi
{
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
    pub struct DVDFileInfo
    {
        pub cb: DVDCommandBlock,

        pub start_addr: u32,
        pub length: u32,
        pub callback: DVDCallback,
    }

    #[repr(C)]
    pub struct OSModuleInfo
    {
        pub id: u32,
        pub next: *mut OSModuleInfo,
        pub prev: *mut OSModuleInfo,

        pub sections_count: u32,
        pub section_info_offset: u32,
        pub name_offset: u32,
        pub name_size: u32,
        pub version: u32,
    }

    #[repr(C)]
    pub union OffsetOrFuncPointer
    {
        pub offset: u32,
        pub func_ptr: Option<unsafe extern "C" fn()>,
    }

    #[repr(C)]
    pub struct OSModuleHeader
    {
        pub mod_info: OSModuleInfo,
        pub bss_size: u32,

        pub reloc_table_offset: u32,
        pub import_table_offset: u32,
        pub import_table_size: u32,

        pub prolog_function_section: u8,
        pub epilog_function_section: u8,
        pub unresolved_function_section: u8,
        pub padding: u8,

        pub prolog_function: OffsetOrFuncPointer,
        pub epilog_function: OffsetOrFuncPointer,
        pub unresolved_function: OffsetOrFuncPointer,
    }

    pub type DVDCallback = extern "C" fn(result: i32, file_info: *mut DVDFileInfo);

    extern "C" {
        pub fn DVDOpen(file_name: *const u8, file_info: *mut DVDFileInfo) -> u8;
        pub fn DVDReadAsyncPrio (file_info: *mut DVDFileInfo, addr: *mut u8, length: u32, offset: u32, callback: DVDCallback, prio: u32) -> u8;
        pub fn DVDClose(file_info: *mut DVDFileInfo) -> u8;

        pub fn OSLink(module: *mut OSModuleInfo, bss: *const u8) -> u8;
    }
}

struct DVDFileInfo(ffi::DVDFileInfo);

impl DVDFileInfo
{
    fn new(filename: &[u8]) -> Self
    {
        let mut fi = MaybeUninit::<ffi::DVDFileInfo>::uninit();
        unsafe {
            ffi::DVDOpen(filename.as_ptr(), fi.as_mut_ptr());
            DVDFileInfo(fi.assume_init())
        }
    }
}

impl Drop for DVDFileInfo
{
    fn drop(&mut self)
    {
        unsafe {
            ffi::DVDClose(&mut self.0);
        }
    }
}

fn leak_slice<'a>(len: usize) -> &'a mut [u8]
{
    unsafe {
        let ptr = alloc(Layout::from_size_align_unchecked(len, 1));
        core::slice::from_raw_parts_mut(ptr, len)
    }
}

// Custom name for section sorting purposes (spaces sort extremely early!)
#[export_name = " rel_loader_hook"]
pub unsafe extern "C" fn rel_loader_hook()
{
    static FLAG: AtomicBool = AtomicBool::new(false);
    extern "C" fn callback(_: i32, _: *mut ffi::DVDFileInfo)
    {
        FLAG.store(true, Ordering::Relaxed);
    }

    let mut fi = DVDFileInfo::new(b"patches.rel\0");
    let rel_size = fi.0.length;

    let rel_data = leak_slice(rel_size as usize);

    ffi::DVDReadAsyncPrio(&mut fi.0, rel_data.as_mut_ptr(), rel_size, 0, callback, 0);

    while FLAG.load(Ordering::Relaxed) == false { }

    let rel_header = rel_data.as_mut_ptr() as *mut ffi::OSModuleHeader;
    let bss_data = leak_slice((&*rel_header).bss_size as usize);

    ffi::OSLink(&mut (*rel_header).mod_info, bss_data.as_mut_ptr());
    if let Some(prolog_ptr) = (*rel_header).prolog_function.func_ptr {
        prolog_ptr();
    }
}

