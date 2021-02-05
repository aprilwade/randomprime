use std::{
    marker::PhantomData,
    borrow::Borrow,
    convert::Infallible,
};

use crate::{
    reader::{Reader, Readable},
    writer::{Writable, Writer},
};

/// Derivable Array Proxy - wraps an iterator for derived array.
///
/// The Readable::size of the wrapper is equal to the sum of all the items in
/// the wrapped iterator. Similarly, when using Writable::write, it calls each
/// the write method of each item in the wrapped iterator.
#[derive(Clone)]
pub struct Dap<I, T>(I, PhantomData<*const T>)
    where I: Iterator + Clone,
          I::Item: Borrow<T>;

impl<I, T> Dap<I, T>
    where I: Iterator + Clone,
          I::Item: Borrow<T>,
{
    pub fn new(i: I) -> Dap<I, T>
    {
        Dap(i, PhantomData)
    }
}

impl<R, I, T> Readable<R> for Dap<I, T>
    where I: Iterator + Clone,
          I::Item: Borrow<T>,
          R: Reader,
          T: Readable<R>,
{
    type Args = Infallible;
    fn read_from(_: &mut R, never: Infallible) -> Result<Self, R::Error>
    {
        match never { }
    }

    fn size(&self) -> Result<usize, R::Error>
    {
        self.0.clone().map(|t| t.borrow().size()).sum()
    }
}

impl<W, I, T> Writable<W> for Dap<I, T>
    where I: Iterator + Clone,
          I::Item: Borrow<T>,
          W: Writer,
          T: Writable<W>
{
    fn write_to(&self, writer: &mut W) -> Result<u64, W::Error>
    {
        let mut s = 0;
        for e in self.0.clone() {
            s += e.borrow().write_to(writer)?
        }
        Ok(s)
    }
}

impl<I, T> From<I> for Dap<I, T>
    where I: Iterator + Clone,
          I::Item: Borrow<T>,
{
    fn from(iter: I) -> Self
    {
        Dap::new(iter)
    }
}

pub trait DerivableFromIterator
{
    type Item;
}
