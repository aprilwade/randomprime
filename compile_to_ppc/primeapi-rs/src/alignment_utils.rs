use generic_array::{ArrayLength, GenericArray};

use core::mem::MaybeUninit;


#[repr(align(32))]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Aligned32<T>(T);

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Aligned32Slice<'a, T>(&'a [T]);
#[derive(Debug, Eq, PartialEq)]
pub struct Aligned32SliceMut<'a, T>(&'a mut [T]);

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
{
    pub fn as_inner_slice<'a, R>(&'a self) -> Aligned32Slice<'a, R>
        where T: TrustedDerefSlice + AsRef<[R]>,
    {
        Aligned32Slice(self.0.as_ref())
    }

    pub fn as_inner_slice_mut<'a, R>(&'a mut self) -> Aligned32SliceMut<'a, R>
        where T: TrustedDerefSlice + AsMut<[R]>,
    {
        Aligned32SliceMut(self.0.as_mut())
    }

}

impl<T> Aligned32<T>
{
    pub const fn new(t: T) -> Aligned32<T>
    {
        Aligned32(t)
    }

    pub fn into_inner(self) -> T
    {
        self.0
    }
    pub fn as_slice<'a>(&'a self) -> Aligned32Slice<'a, T>
    {
        Aligned32Slice(core::slice::from_ref(&self.0))
    }

    pub fn as_slice_mut<'a>(&'a mut self) -> Aligned32SliceMut<'a, T>
    {
        Aligned32SliceMut(core::slice::from_mut(&mut self.0))
    }
}

impl<T> core::ops::Deref for Aligned32<T>
{
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &Self::Target
    {
        &self.0
    }
}


impl<T> core::ops::DerefMut for Aligned32<T>
{
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target
    {
        &mut self.0
    }
}

impl<'a, T> Aligned32Slice<'a, T>
{
    pub fn empty() -> Self
    {
        Aligned32Slice(&[])
    }

    pub fn truncate_to_len(self, len: usize) -> Aligned32Slice<'a, T>
    {
        Aligned32Slice(&self.0[..len])
    }

    pub fn from_slice(slice: &'a [T]) -> Option<Self>
    {
        if slice.as_ptr() as usize & 31 == 0 {
            Some(Aligned32Slice(slice))
        } else {
            None
        }
    }

    pub unsafe fn from_slice_unchecked(slice: &'a [T]) -> Self
    {
        Aligned32Slice(slice)
    }
}

impl<'a, T> Aligned32Slice<'a, MaybeUninit<T>>
{
    pub unsafe fn assume_init(&self) -> Aligned32Slice<'a, T>
    {
        Aligned32Slice(core::slice::from_raw_parts(self.as_ptr() as *mut T, self.len()))
    }
}

pub unsafe trait Splittable { }
unsafe impl Splittable for u8 { }
unsafe impl Splittable for MaybeUninit<u8>{ }

impl<'a, T> Aligned32Slice<'a, T>
    where T: Splittable
{
    pub fn split_unaligned_prefix(slice: &'a [T]) -> (&'a [T], Self)
    {
        let buf_addr = slice.as_ptr() as usize;
        let aligned_addr = (buf_addr + 31) & !31;
        let unaligned_prefix = core::cmp::min(aligned_addr - buf_addr, slice.len());

        (&slice[..unaligned_prefix], Aligned32Slice(&slice[unaligned_prefix..]))
    }
}

pub fn empty_aligned_slice() -> Aligned32Slice<'static, u8>
{
    Aligned32Slice::empty()
}

impl<'a, T> Aligned32SliceMut<'a, T>
{
    pub fn empty() -> Self
    {
        Aligned32SliceMut(&mut [])
    }

    pub fn truncate_to_len(self, len: usize) -> Aligned32SliceMut<'a, T>
    {
        Aligned32SliceMut(&mut self.0[..len])
    }

    pub fn reborrow<'b>(&'b mut self) -> Aligned32SliceMut<'b, T>
    {
        Aligned32SliceMut(self.0)
    }

    pub fn from_slice(slice: &'a mut [T]) -> Option<Self>
    {
        if slice.as_mut_ptr() as usize & 31 == 0 {
            Some(Aligned32SliceMut(slice))
        } else {
            None
        }
    }

    pub unsafe fn from_slice_unchecked(slice: &'a mut [T]) -> Self
    {
        Aligned32SliceMut(slice)
    }
}

impl<'a, T> Aligned32SliceMut<'a, MaybeUninit<T>>
{
    pub unsafe fn assume_init<'b>(&'b mut self) -> Aligned32SliceMut<'b, T>
    {
        let len = self.len();
        Aligned32SliceMut(core::slice::from_raw_parts_mut(self.as_mut_ptr() as *mut T, len))
    }
}

impl<'a, T> Aligned32SliceMut<'a, T>
    where T: Splittable
{
    pub fn split_unaligned_prefix(slice: &'a mut [T]) -> (&'a mut [T], Self)
    {
        let buf_addr = slice.as_ptr() as usize;
        let aligned_addr = (buf_addr + 31) & !31;
        let unaligned_prefix = core::cmp::min(aligned_addr - buf_addr, slice.len());

        let (unaligned, aligned) = slice.split_at_mut(unaligned_prefix);
        (unaligned, Aligned32SliceMut(aligned))
    }
}

pub fn empty_aligned_slice_mut() -> Aligned32SliceMut<'static, u8>
{
    Aligned32SliceMut::empty()
}

impl<'a, T> core::ops::Deref for Aligned32Slice<'a, T>
{
    type Target = [T];
    #[inline(always)]
    fn deref(&self) -> &Self::Target
    {
        &self.0
    }
}

impl<'a, T> core::ops::Deref for Aligned32SliceMut<'a, T>
{
    type Target = [T];
    #[inline(always)]
    fn deref(&self) -> &Self::Target
    {
        &self.0
    }
}


impl<'a, T> core::ops::DerefMut for Aligned32SliceMut<'a, T>
{
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target
    {
        &mut self.0
    }
}

#[derive(Debug, Copy, Clone)]
pub struct EmptyArray;

impl<T> Into<GenericArray<T, generic_array::typenum::U0>> for EmptyArray
{
    fn into(self) -> GenericArray<T, generic_array::typenum::U0>
    {
        generic_array::arr![T;]
    }
}
