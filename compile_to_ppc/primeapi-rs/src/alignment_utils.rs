use generic_array::{ArrayLength, GenericArray};

use core::cmp;
use core::mem::MaybeUninit;
use core::ops::{Index, IndexMut, RangeTo};
use core::slice;


#[repr(align(32), C)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Aligned32<T: ?Sized>(T);

pub trait TrustedDerefSlice { }


impl<T, N: ArrayLength<T>> TrustedDerefSlice for GenericArray<T, N> { }

macro_rules! trusted_deref_slice_array {
    ($($e:tt)*) => {
        $(
            impl<T> TrustedDerefSlice for [T; $e] { }
        )*
    }
}
trusted_deref_slice_array!(0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19);
trusted_deref_slice_array!(20 21 22 23 24 25 26 27 28 29 30 31 32 33 34 35 36 37 38 39);
trusted_deref_slice_array!(40 41 42 43 44 45 46 47 48 49 50 51 52 53 54 55 56 57 58 59);
trusted_deref_slice_array!(60 61 62 63 64);

impl<T> Aligned32<T>
    where T: TrustedDerefSlice,
{
    #[inline(always)]
    pub fn as_slice<'a, R>(&'a self) -> &'a Aligned32<[R]>
        where T: AsRef<[R]>,
    {
        unsafe { Aligned32::from_ref_unchecked(self.0.as_ref()) }
    }

    #[inline(always)]
    pub fn as_slice_mut<'a, R>(&'a mut self) -> &'a mut Aligned32<[R]>
        where T: AsMut<[R]>,
    {
        unsafe { Aligned32::from_mut_unchecked(self.0.as_mut()) }
    }
}

impl<T> Aligned32<T>
{
    #[inline(always)]
    pub const fn new(t: T) -> Aligned32<T>
    {
        Aligned32(t)
    }

    #[inline(always)]
    pub fn into_inner(self) -> T
    {
        self.0
    }

    #[inline(always)]
    pub fn as_unit_slice<'a>(&'a self) -> &'a Aligned32<[T]>
    {
        unsafe { Aligned32::from_ref_unchecked(slice::from_ref(&self.0)) }
    }

    #[inline(always)]
    pub fn as_unit_slice_mut<'a>(&'a mut self) -> &'a mut Aligned32<[T]>
    {
        unsafe { Aligned32::from_mut_unchecked(slice::from_mut(&mut self.0)) }
    }
}

impl<T> Aligned32<[T]>
{
    #[inline(always)]
    pub fn empty<'a>() -> &'a Self
    {
        unsafe { Aligned32::from_ref_unchecked(&[]) }
    }

    #[inline(always)]
    pub fn empty_mut<'a>() -> &'a mut Self
    {
        unsafe { Aligned32::from_mut_unchecked(&mut []) }
    }

}

impl<T: ?Sized> Aligned32<T>
{
    pub fn from_ref<'a>(t: &'a T) -> Option<&'a Self>
    {
        if t as *const _ as *const u8 as usize & 31 == 0 {
            Some(unsafe { Aligned32::from_ref_unchecked(t) })
        } else {
            None
        }
    }

    pub fn from_mut<'a>(t: &'a mut T) -> Option<&'a mut Self>
    {
        if t as *const _ as *const u8 as usize & 31 == 0 {
            Some(unsafe { Aligned32::from_mut_unchecked(t) })
        } else {
            None
        }
    }

    #[inline(always)]
    pub unsafe fn from_ref_unchecked<'a>(t: &'a T) -> &'a Self
    {
        &*(t as *const T as *const Self)
    }

    #[inline(always)]
    pub unsafe fn from_mut_unchecked<'a>(t: &'a mut T) -> &'a mut Self
    {
        &mut *(t as *mut T as *mut Self)
    }
}

impl<T> Aligned32<MaybeUninit<T>>
{
    #[inline(always)]
    pub unsafe fn assume_init(self) -> Aligned32<T>
    {
        Aligned32(self.0.assume_init())
    }
}

impl<T> Aligned32<[MaybeUninit<T>]>
{
    #[inline(always)]
    pub unsafe fn assume_init(&self) -> &Aligned32<[T]>
    {
        Aligned32::from_ref_unchecked(&*(&self.0 as *const [MaybeUninit<T>] as *const [T]))
    }

    #[inline(always)]
    pub unsafe fn assume_init_mut(&mut self) -> &mut Aligned32<[T]>
    {
        Aligned32::from_mut_unchecked(&mut *(&mut self.0 as *mut [MaybeUninit<T>] as *mut [T]))
    }
}


impl<T: ?Sized> core::ops::Deref for Aligned32<T>
{
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &Self::Target
    {
        &self.0
    }
}

impl<T: ?Sized> core::ops::DerefMut for Aligned32<T>
{
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target
    {
        &mut self.0
    }
}

impl<T> Index<RangeTo<usize>> for Aligned32<[T]>
{
    type Output = Aligned32<[T]>;
    #[inline(always)]
    fn index(&self, index: RangeTo<usize>) -> &Self::Output
    {
        unsafe { Aligned32::from_ref_unchecked(&self.0[index]) }
    }
}

impl<T> IndexMut<RangeTo<usize>> for Aligned32<[T]>
{
    #[inline(always)]
    fn index_mut(&mut self, index: RangeTo<usize>) -> &mut Self::Output
    {
        unsafe { Aligned32::from_mut_unchecked(&mut self.0[index]) }
    }
}

pub unsafe trait Splittable { }
unsafe impl Splittable for u8 { }
unsafe impl Splittable for MaybeUninit<u8>{ }

impl<T> Aligned32<[T]>
    where T: Splittable
{
    pub fn split_unaligned_prefix<'a>(slice: &'a [T]) -> (&'a [T], &'a Self)
    {
        let buf_addr = slice.as_ptr() as usize;
        let aligned_addr = (buf_addr + 31) & !31;
        let unaligned_prefix = cmp::min(aligned_addr - buf_addr, slice.len());

        let (unaligned, aligned) = slice.split_at(unaligned_prefix);
        (unaligned, unsafe { Aligned32::from_ref_unchecked(aligned) })
    }

    pub fn split_unaligned_prefix_mut<'a>(slice: &'a mut [T]) -> (&'a mut [T], &'a mut Self)
    {
        let buf_addr = slice.as_ptr() as usize;
        let aligned_addr = (buf_addr + 31) & !31;
        let unaligned_prefix = cmp::min(aligned_addr - buf_addr, slice.len());

        let (unaligned, aligned) = slice.split_at_mut(unaligned_prefix);
        (unaligned, unsafe { Aligned32::from_mut_unchecked(aligned) })
    }
}

#[derive(Debug, Copy, Clone)]
pub struct EmptyArray;

impl<T> Into<GenericArray<T, generic_array::typenum::U0>> for EmptyArray
{
    #[inline(always)]
    fn into(self) -> GenericArray<T, generic_array::typenum::U0>
    {
        generic_array::arr![T;]
    }
}
