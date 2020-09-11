#![feature(alloc_error_handler)]
#![no_std]

extern crate alloc;

use linkme::distributed_slice;

use ufmt::uwriteln;

// Rexport these macros
pub use primeapi_macros::{cpp_method, cw_link_name, patch_fn, prolog_fn};

use core::alloc::{GlobalAlloc, Layout};
use core::convert::Infallible;
use core::ffi::c_void;

pub mod rstl;
pub mod dol_sdk {
    pub mod dvd;
    pub mod os;
}
pub mod mp1;
pub mod alignment_utils;

#[doc(hidden)]
pub mod reexport {
    pub use paste;
}

#[macro_export]
macro_rules! cpp_field {
    ($id:ident: $ty:ty; ptr @ $e:expr) => {
        #[inline(always)]
        pub fn $id(this: *const Self) -> *const $ty
        {
            (this as usize + ($e)) as *mut _
        }

        $crate::reexport::paste::item! {
            #[inline(always)]
            pub fn [<$id _mut>](this: *mut Self) -> *mut $ty
            {
                (this as usize + ($e)) as *mut _
            }
        }
    };
    ($id:ident: $ty:ty; ro_val @ $e:expr) => {
        #[inline(always)]
        pub unsafe fn $id(this: *const Self) -> $ty
        {
            core::ptr::read((this as usize + ($e)) as *const $ty)
        }
    };
    ($id:ident: $ty:ty; val @ $e:expr) => {
        $crate::cpp_field!($id: $ty; ro_val @ $e);
        $crate::reexport::paste::item! {
            #[inline(always)]
            pub unsafe fn [<set_ $id>](this: *const Self, val: $ty)
            {
                core::ptr::write((this as usize + ($e)) as *mut $ty, val)
            }
        }
    };
}

extern "C"
{
    fn fwrite(bytes: *const u8, len: usize, count: usize, fd: *const u32) -> usize;

    pub fn printf(fmt: *const u8, ...);

    pub fn sprintf(s: *mut u8, fmt: *const u8, ...);
    // #[link_name = "__nw__FUlPCcPCc"]
    #[cw_link_name(operator new(unsigned long, const char *, const char *))]
    fn operator_new(len: usize, loc: *const u8, type_: *const u8) -> *mut c_void;

    // #[link_name = "Free__7CMemoryFPCv"]
    #[cw_link_name(CMemory::Free(const void *))]
    fn free(ptr: *const c_void);
}

#[macro_export]
macro_rules! dbg {
    ($($tts:tt)*) => { { } }
}

// #[macro_export]
// macro_rules! dbg {
//     () => {{
//         use core::fmt::Write;
//         let _ = core::writeln!($crate::Mp1Stdout, "[{}:{}]", file!(), line!());
//     }};
//     ($val:expr) => {{
//         // Use of `match` here is intentional because it affects the lifetimes
//         // of temporaries - https://stackoverflow.com/a/48732525/1063961
//         use core::fmt::Write;
//         match $val {
//             tmp => {
//                 let _ = core::writeln!($crate::Mp1Stdout, "[{}:{}] {} = {:#?}",
//                     file!(), line!(), stringify!($val), &tmp);
//                 tmp
//             }
//         }
//     }};
//     // Trailing comma with single argument is ignored
//     ($val:expr,) => { $crate::dbg!($val) };
//     ($($val:expr),+ $(,)?) => {
//         ($($crate::dbg!($val)),+,)
//     };
// }

pub unsafe fn malloc(len: usize) -> *mut c_void
{
    operator_new(len, b"??\0".as_ptr(), b"??\0".as_ptr())
}

struct Mp1Allocator;

unsafe impl GlobalAlloc for Mp1Allocator
{
    unsafe fn alloc(&self, layout: Layout) -> *mut u8
    {
        malloc(layout.size()) as *mut u8
    }
    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout)
    {
        free(ptr as *const c_void)
    }
}

#[global_allocator]
static A: Mp1Allocator = Mp1Allocator;


pub struct Mp1Stdout;

impl core::fmt::Write for Mp1Stdout
{
    fn write_str(&mut self, s: &str) -> Result<(), core::fmt::Error>
    {
        unsafe {
            // TODO: Check result?
            // printf(b"%s\0".as_ptr(), s.as);
            // printf(b"test %d\n\0".as_ptr(), s.len());
            fwrite(s.as_bytes().as_ptr(), s.len(), 1, 0x803f27c8 as *const _);
        }
        Ok(())
    }
}


impl ufmt::uWrite for Mp1Stdout
{
    type Error = Infallible;
    fn write_str(&mut self, s: &str) -> Result<(), Self::Error>
    {
        unsafe {
            // TODO: Check result?
            // printf(b"%s\0".as_ptr(), s.as);
            // printf(b"test %d\n\0".as_ptr(), s.len());
            fwrite(s.as_bytes().as_ptr(), s.len(), 1, 0x803f27c8 as *const _);
        }
        Ok(())
    }
}


fn halt() -> !
{
    // extern "C" {
    //     fn PPCHalt() -> !;
    // }
    // unsafe {
    //     PPCHalt()
    // }
    loop {}
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    if cfg!(debug_assertions) {
        if let Some(loc) = info.location() {
            uwriteln!(Mp1Stdout, "Panic at {}:{}", loc.file(), loc.line()).ok();
        } else {
            uwriteln!(Mp1Stdout, "Panic").ok();
        }
        if let Some(msg) = info.payload().downcast_ref::<&str>() {
            uwriteln!(Mp1Stdout, "msg: {}", msg).ok();
        }
    }

    halt()
}

#[alloc_error_handler]
fn alloc_error(_layout: Layout) -> ! {
    if cfg!(debug_assertions) {
        uwriteln!(Mp1Stdout, "Alloc failed").ok();
    }

    halt()
}


#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PatchKind
{
    Call,
    Return,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Patch
{
    fn_ptr_to_patch: *const u8,
    patch_offset: usize,
    target_fn_ptr: *const u8,
    kind: PatchKind,
}

impl Patch
{
    pub const fn call_patch(
        fn_ptr_to_patch: *const u8,
        patch_offset: usize,
        target_fn_ptr: *const u8
    ) -> Patch
    {
        Patch {
            fn_ptr_to_patch,
            patch_offset,
            target_fn_ptr,
            kind: PatchKind::Call,
        }
    }

    pub const fn return_patch(
        fn_ptr_to_patch: *const u8,
        patch_offset: usize,
        target_fn_ptr: *const u8
    ) -> Patch
    {
        Patch {
            fn_ptr_to_patch,
            patch_offset,
            target_fn_ptr,
            kind: PatchKind::Return,
        }
    }
}

unsafe impl Sync for Patch { }

#[distributed_slice]
pub static PATCHES: [Patch] = [..];


#[distributed_slice]
pub static PROLOG_FUNCS: [unsafe extern "C" fn()] = [..];

#[cfg(feature = "rel_prolog")]
#[no_mangle]
unsafe extern "C" fn __rel_prolog()
{
    printf(b"prolog called\n\0".as_ptr());
    for patch in PATCHES.iter() {
        let instr_ptr = patch.fn_ptr_to_patch.add(patch.patch_offset) as *mut u32;
        let instr = core::ptr::read(instr_ptr);

        let bounds_check_and_mask = |len: u8, addr: i64| {
            // XXX Only len + 1 because this is a sign-extended value
            debug_assert!(!(
                    addr > (1 << (len + 1)) - 1
                    || addr < -1 << (len + 1)
                    || addr as u64 & 0x3 != 0));

            (addr as u64 & ((1 << (len + 2)) - 1)) as u32
        };

        let instr = match patch.kind {
            PatchKind::Call => {
                let rel_addr = patch.target_fn_ptr as i64 - instr_ptr as i64;
                let imm = bounds_check_and_mask(24, rel_addr);
                ((instr & 0xfc000003) | imm)
            },
            PatchKind::Return => {
                // Assert the instr is actually a return
                debug_assert_eq!(instr, 0x4e800020);

                let rel_addr = patch.target_fn_ptr as i64 - instr_ptr as i64;
                let imm = bounds_check_and_mask(24, rel_addr);
                (0x48000000 | imm) // Uncondtional jump
            },
        };

        core::ptr::write(instr_ptr, instr);
    }

    for prolog_func in PROLOG_FUNCS.iter() {
        printf(b"calling prolog func ptr %p\n\0".as_ptr(), *prolog_func);
        prolog_func();
    }
}


// TODO: Maybe re-enable this later? The core::fmt machinery seems to need it sometimes
#[no_mangle]
unsafe extern "C" fn bcmp(mut b1: *const u8, mut b2: *const u8, mut len: u32) -> u32
{
    if len == 0 {
        return 0
    }

    while len > 0 {
        if core::ptr::read(b1) != core::ptr::read(b2) {
            break
        }

        b1 = b1.offset(1);
        b2 = b2.offset(1);
        len -= 1;
    }

    len
}
