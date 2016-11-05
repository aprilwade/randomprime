
use std::mem;
use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::io::Write;
use std::fmt::{Debug, Formatter, Error};

use reader::{Reader, Readable};
use writer::Writable;

struct LazyCell<A, T>(UnsafeCell<LazyCell_<A, T>>, );

enum LazyCell_<A, T>
{
    Unintialized(A),
    Initializing,
    Initialized(T),
}

const ERR: &'static str = "Access to an initializing LazyCell is not permitted";
impl<A, T> LazyCell<A, T>
{
    fn new(a: A) -> LazyCell<A, T>
    {
        LazyCell(UnsafeCell::new(LazyCell_::Unintialized(a)))
    }

    fn initialize<F>(&self, f: F) -> &mut T
        where F: FnOnce(A) -> T
    {
        let mut data = LazyCell_::Initializing;
        mem::swap(unsafe { &mut *self.0.get() }, &mut data);

        let mut data = match data {
            LazyCell_::Initializing | LazyCell_::Initialized(_) => unreachable!(),
            LazyCell_::Unintialized(a) => LazyCell_::Initialized(f(a)),
        };
        mem::swap(&mut data, unsafe { &mut *self.0.get() });
        match unsafe { &mut *self.0.get() } {
            &mut LazyCell_::Initialized(ref mut t) => t,
            _ => unreachable!(),
        }
    }

    fn borrow<F>(&self, f: F) -> &T
        where F: FnOnce(A) -> T
    {
        match unsafe { &*self.0.get() } {
            &LazyCell_::Initializing => panic!(ERR),
            &LazyCell_::Initialized(ref t) => t,
            &LazyCell_::Unintialized(_) => self.initialize(f),
        }
    }

    fn borrow_mut<F>(&mut self, f: F) -> &mut T
        where F: FnOnce(A) -> T
    {
        match unsafe { &mut *self.0.get() } {
            &mut LazyCell_::Initializing => panic!(ERR),
            &mut LazyCell_::Initialized(ref mut t) => t,
            &mut LazyCell_::Unintialized(_) => self.initialize(f),
        }
    }

    unsafe fn uninitialized_data(&self) -> Option<&A>
    {
        match  &*self.0.get() {
            &LazyCell_::Initializing => panic!(ERR),
            &LazyCell_::Initialized(_) => None,
            &LazyCell_::Unintialized(ref a) => Some(a),
        }
    }
}

impl<A, T> Debug for LazyCell<A, T>
    where A: Debug,
          T: Debug,
{
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), Error>
    {
        match unsafe { &*self.0.get() } {
            &LazyCell_::Initializing => panic!(ERR),
            &LazyCell_::Initialized(ref t) =>
                formatter.write_fmt(format_args!("Initialized {:?}", t)),
            &LazyCell_::Unintialized(ref a) =>
                formatter.write_fmt(format_args!("Unintialized {:?}", a)),
        }
    }
}

/// May only be used on fixed sized objects.
#[derive(Debug)]
pub struct Lazy<'a, T>
    where T: Readable<'a>
{
    cell: LazyCell<(Reader<'a>, T::Args), T>
}

impl<'a, T> Deref for Lazy<'a, T>
    where T: Readable<'a>
{
    type Target = T;
    #[inline]
    fn deref(&self) -> &T
    {
        self.cell.borrow(|(mut reader, args)| reader.read(args))
    }
}

impl<'a, T> DerefMut for Lazy<'a, T>
    where T: Readable<'a>
{
    #[inline]
    fn deref_mut(&mut self) -> &mut T
    {
        self.cell.borrow_mut(|(mut reader, args)| reader.read(args))
    }
}

impl<'a, T> Readable<'a> for Lazy<'a, T>
    where T: Readable<'a>
{
    type Args = T::Args;

    #[inline]
    fn read(reader: Reader<'a>, args: Self::Args) -> (Self, Reader<'a>)
    {
        let lazy = Lazy { cell: LazyCell::new((reader.clone(), args)) };
        let size = Readable::size(&lazy);
        (lazy, reader.offset(size))
    }

    #[inline]
    fn size(&self) -> usize
    {
        const ERR: &'static str = "LazyCell may only be used with fixed-sized readable objects.";
        Self::fixed_size().expect(ERR)
    }

    #[inline]
    fn fixed_size() -> Option<usize>
    {
        T::fixed_size()
    }
}


impl<'a, T> Writable for Lazy<'a, T>
    where T: Readable<'a> + Writable
{
    fn write<W: Write>(&self, writer: &mut W)
    {
        unsafe { 
            if let Some(uninit_data) = self.cell.uninitialized_data() {
                writer.write(&(*uninit_data.0)[0..self.size()]).unwrap();
            } else {
                self.deref().write(writer)
            }
        }
    }
}

#[derive(Debug)]
pub struct LazySized<'a, T>
    where T: Readable<'a>
{
    cell: LazyCell<(Reader<'a>, T::Args), T>
}

impl<'a, T> Deref for LazySized<'a, T>
    where T: Readable<'a>
{
    type Target = T;
    #[inline]
    fn deref(&self) -> &T
    {
        self.cell.borrow(|(mut reader, args)| reader.read(args))
    }
}

impl<'a, T> DerefMut for LazySized<'a, T>
    where T: Readable<'a>
{
    #[inline]
    fn deref_mut(&mut self) -> &mut T
    {
        self.cell.borrow_mut(|(mut reader, args)| reader.read(args))
    }
}

impl<'a, T> Readable<'a> for LazySized<'a, T>
    where T: Readable<'a>
{
    type Args = (usize, T::Args);

    #[inline]
    fn read(reader: Reader<'a>, (size, args): Self::Args) -> (Self, Reader<'a>)
    {
        let lazy = LazySized { cell: LazyCell::new((reader.truncated(size), args)) };
        (lazy, reader.offset(size))
    }

    #[inline]
    fn size(&self) -> usize
    {
        unsafe {
            if let Some(uninit_data) = self.cell.uninitialized_data() {
                uninit_data.0.len()
            } else {
                self.deref().size()
            }
        }
    }
}

impl<'a, T> Writable for LazySized<'a, T>
    where T: Readable<'a> + Writable
{
    fn write<W: Write>(&self, writer: &mut W)
    {
        unsafe {
            if let Some(uninit_data) = self.cell.uninitialized_data() {
                writer.write(*uninit_data.0).unwrap();
            } else {
                self.deref().write(writer);
            }
        }
    }
}

#[cfg(test)]
mod tests
{
    use ::{LazySized, Reader, Array, Readable};
    #[test]
    fn test_lazy_cell()
    {
        let data = [0xFC, 0xFD, 0xFE, 0xFF];
        let reader = Reader::new(&data);
        let mut lazy : LazySized<Array<u8>> = LazySized::read(reader, (4, (4, ()))).0;
        let array = lazy.as_mut_slice();
        assert_eq!(array[0], 0xFC);
        assert_eq!(array[1], 0xFD);
        assert_eq!(array[2], 0xFE);
        assert_eq!(array[3], 0xFF);
    }
}
