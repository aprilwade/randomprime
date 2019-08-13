#![feature(alloc_error_handler)]
#![feature(macros_in_extern)]
#![no_std]

use linkme::distributed_slice;

// Rexport these macros
pub use patch_fn_macros::{patch_fn, cw_link_name};

use core::alloc::{GlobalAlloc, Layout};
use core::ffi::c_void;
use core::fmt::{self, Write as _};

extern "C"
{
    fn fwrite(bytes: *const u8, len: usize, count: usize) -> usize;

    #[allow(unused)]
    fn printf(fmt: *const u8, ...);

    // #[link_name = "__nw__FUlPCcPCc"]
    #[cw_link_name(operator new(unsigned long, const char *, const char *))]
    fn operator_new(len: usize, loc: *const u8, type_: *const u8) -> *mut c_void;

    // #[link_name = "Free__7CMemoryFPCv"]
    #[cw_link_name(CMemory::Free(const void *))]
    fn cmemory_free(ptr: *const c_void);
}

struct Mp1Allocator;

unsafe impl GlobalAlloc for Mp1Allocator
{
    unsafe fn alloc(&self, layout: Layout) -> *mut u8
    {
        operator_new(layout.size(), b"??\0".as_ptr(), b"??\0".as_ptr()) as *mut u8
    }
    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout)
    {
        cmemory_free(ptr as *const c_void)
    }
}

#[global_allocator]
static A: Mp1Allocator = Mp1Allocator;

pub struct Mp1Stdout;

impl fmt::Write for Mp1Stdout
{
    fn write_str(&mut self, s: &str) -> fmt::Result
    {
        unsafe {
            // TODO: Check result?
            fwrite(s.as_bytes().as_ptr(), s.len(), 1);
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
        writeln!(Mp1Stdout, "{}", info).ok();
    }

    halt()
}

#[alloc_error_handler]
fn alloc_error(_layout: Layout) -> ! {
    if cfg!(debug_assertions) {
        writeln!(Mp1Stdout, "Alloc failed").ok();
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

#[no_mangle]
unsafe extern "C" fn __rel_prolog()
{
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
}

// TODO: Maybe re-enable this later? The core::fmt machinery seems to need it sometimes
// #[no_mangle]
// unsafe extern "C" fn bcmp(mut b1: *const u8, mut b2: *const u8, mut len: u32) -> u32
// {
//     if len == 0 {
//         return 0
//     }

//     while len > 0 {
//         if ptr::read(b1) != ptr::read(b2) {
//             break
//         }

//         b1 = b1.offset(1);
//         b2 = b2.offset(1);
//         len -= 1;
//     }

//     len
// }
