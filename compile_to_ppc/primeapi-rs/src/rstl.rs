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


#[repr(C)]
pub struct Vector<T>
{
    // XXX Is _first actually used? primeapi doesn't seem to think so
    _first: usize,
    size: usize,
    capacity: usize,
    data: *mut T,
}

impl<T> Vector<T>
{
    pub fn size(&self) -> usize
    {
        self.size
    }

    pub fn capacity(&self) -> usize
    {
        self.capacity
    }
}

impl<T> core::ops::Deref for Vector<T>
{
    type Target = [T];
    fn deref(&self) -> &Self::Target
    {
        unsafe {
            core::slice::from_raw_parts(self.data, self.size)
        }
    }
}

impl<T> core::ops::DerefMut for Vector<T>
{
    fn deref_mut(&mut self) -> &mut Self::Target
    {
        unsafe {
            core::slice::from_raw_parts_mut(self.data, self.size)
        }
    }
}

impl<T> Into<alloc::vec::Vec<T>> for Vector<T>
{
    fn into(self) -> alloc::vec::Vec<T>
    {
        unsafe {
            alloc::vec::Vec::from_raw_parts(
                self.data,
                self.size,
                self.capacity,
            )
        }
    }
}

impl<T> From<alloc::vec::Vec<T>> for Vector<T>
{
    fn from(vec: alloc::vec::Vec<T>) -> Self
    {
        Vector {
            _first: 0,
            size: vec.len(),
            capacity: vec.capacity(),
            data: alloc::boxed::Box::into_raw(vec.into_boxed_slice()) as *mut T,
        }
    }
}
