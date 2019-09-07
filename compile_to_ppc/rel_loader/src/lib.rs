#![no_std]

use primeapi::dol_sdk::dvd::DVDFileInfo;
use primeapi::dol_sdk::os::{OSLink, OSModuleHeader};

include!("../../patches_config.rs");
#[no_mangle]
pub static REL_CONFIG: RelConfig = RelConfig {
    quickplay_mlvl: 0xFFFFFFFF,
    quickplay_mrea: 0xFFFFFFFF,
};


#[inline(always)]
fn leak_slice<'a>(len: usize) -> &'a mut [u8]
{
    unsafe {
        let ptr = primeapi::malloc(len) as *mut u8;
        core::slice::from_raw_parts_mut(ptr, len)
    }
}

#[export_name = "rel_loader_hook"]
pub unsafe extern "C" fn rel_loader_hook()
{
    let mut fi = DVDFileInfo::new(b"patches.rel\0");
    let rel_size = fi.file_length();

    let rel_data = leak_slice(rel_size as usize);

    {
        let _handle = fi.read_async(rel_data, 0, 0);
    }

    let rel_header = rel_data.as_mut_ptr() as *mut OSModuleHeader;
    let bss_data = leak_slice((&*rel_header).bss_size as usize);

    OSLink(&mut (*rel_header).mod_info, bss_data.as_mut_ptr());
    if let Some(prolog_ptr) = (*rel_header).prolog_function.func_ptr {
        prolog_ptr();
    }
}

