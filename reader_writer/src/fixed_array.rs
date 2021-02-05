use crate::{
    derivable_array_proxy::DerivableFromIterator,
    generic_array::{GenericArray, ArrayLength},
    reader::{Readable, Reader, ReaderEx},
    writer::{Writable, Writer},
};

pub type FixedArray<T, N> = GenericArray<T, N>;

impl<R, T, N> Readable<R> for FixedArray<T, N>
    where N: ArrayLength<T>,
          R: Reader,
          T: Readable<R>,
          T::Args: Clone,
{
    type Args = T::Args;
    fn read_from(reader: &mut R, args: Self::Args) -> Result<Self, R::Error>
    {
        (0..N::to_usize())
            .map(|_| reader.read(args.clone()))
            .collect()
    }

    fn size(&self) -> Result<usize, R::Error>
    {
        if let Some(fs) = <Self as Readable<R>>::fixed_size() {
            Ok(fs)
        } else {
            self.iter()
                .try_fold(0, |s, i| Ok(s + i.size()?))
        }
    }


    fn fixed_size() -> Option<usize>
    {
        T::fixed_size().map(|i| i * N::to_usize())
    }
}

impl<W, T, N> Writable<W> for FixedArray<T, N>
    where N: ArrayLength<T>,
          W: Writer,
          T: Writable<W>,
{
    fn write_to(&self, writer: &mut W) -> Result<u64, W::Error>
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
