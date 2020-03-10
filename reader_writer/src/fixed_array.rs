use std::io;

use crate::{
    derivable_array_proxy::DerivableFromIterator,
    generic_array::{GenericArray, ArrayLength},
    reader::{Reader, Readable},
    writer::Writable,
};

pub type FixedArray<T, N> = GenericArray<T, N>;

impl<'r, T, N> Readable<'r> for FixedArray<T, N>
    where N: ArrayLength<T>,
          T: Readable<'r>,
          T::Args: Clone,
{
    type Args = T::Args;


    fn read_from(reader: &mut Reader<'r>, args: Self::Args) -> Self
    {
        let array = {
            let iter = (0..N::to_usize()).map(|_| reader.read(args.clone()));
            GenericArray::from_exact_iter(iter).unwrap()
        };
        array
    }

    fn size(&self) -> usize
    {
        <Self as Readable>::fixed_size()
            .unwrap_or_else(|| self.iter().fold(0, |s, i| s + i.size()))
    }


    fn fixed_size() -> Option<usize>
    {
        T::fixed_size().map(|i| i * N::to_usize())
    }
}

impl<'r, T, N> Writable for FixedArray<T, N>
    where N: ArrayLength<T>,
          T: Readable<'r> + Writable,
          T::Args: Clone,
{
    fn write_to<W: io::Write>(&self, writer: &mut W) -> io::Result<u64>
    {
        let mut s = 0;
        for elem in self.iter() {
            s += elem.write_to(writer)?
        }
        Ok(s)
    }
}

impl<T, N> DerivableFromIterator for FixedArray<T, N>
    where N: ArrayLength<T>,
{
        type Item = T;
}
