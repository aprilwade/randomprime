use core::mem::MaybeUninit;
use crate::cw_link_name;

#[repr(C)]
struct WStringCowData
{
    capacity: u32,
    ref_count: u32,
    // data: [u16],
}

#[repr(C)]
pub struct WString
{
    data: *mut u16,
    cow: *mut WStringCowData,
    size: usize,
}

extern "C" {
    // wstring_l__4rstlFPCw
    #[cw_link_name(rstl::wstring_l(const wchar_t *))]
    fn wstr_ctor(this: *mut WString, s: *const u16) -> *mut WString;

    // internal_dereference__Q24rstl66basic_string<w,Q24rstl14char_traits<w>,Q24rstl17rmemory_allocator>Fv
    #[cw_link_name(rstl::basic_string<wchar_t, rstl::char_traits<wchar_t>, rstl::rmemory_allocator>::internal_dereference(void))]
    fn dtor(this: *mut WString);

    // internal_allocate__Q24rstl66basic_string<w,Q24rstl14char_traits<w>,Q24rstl17rmemory_allocator>Fi
    #[cw_link_name(rstl::basic_string<wchar_t, rstl::char_traits<wchar_t>, rstl::rmemory_allocator>::internal_allocate(int))]
    fn ctor_reserve(this: *mut WString, cap: u32);
}

impl WString
{
    pub fn from_ucs2_str(s: *const u16) -> Self
    {
        let mut this = MaybeUninit::uninit();// WString(MaybeUninit::uninit());
        unsafe {
            wstr_ctor(this.as_mut_ptr(), s);
            this.assume_init()
        }
    }

    pub fn with_capacity(cap: usize) -> Self
    {
        let mut this = MaybeUninit::uninit();
        let mut this = unsafe {
            ctor_reserve(this.as_mut_ptr(), cap as u32 + 1);
            this.assume_init()
        };
        this.size = 0;
        this
    }

    pub fn from_ascii(s: &[u8]) -> Self
    {
        let mut this = Self::with_capacity(s.len());
        for (i, b) in s.iter().enumerate() {
            unsafe {
                core::ptr::write(this.data.offset(i as isize), *b as u16);
            }
        }
        unsafe {
            core::ptr::write(this.data.offset(s.len() as isize), 0);
        }
        this.size = s.len();
        this
    }
}

impl Drop for WString
{
    fn drop(&mut self)
    {
        unsafe {
            dtor(self);
        }
    }
}
