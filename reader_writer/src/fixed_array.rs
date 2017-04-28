use std::io;

use reader::{Reader, Readable};
use writer::Writable;
use generic_array::{GenericArray, ArrayLength};

pub type FixedArray<T, N> = GenericArray<T, N>;

// TODO: It would be nice if we could initialize from an iterator directly.
//       That would allow the Default bound to be removed
//       That would require an upstream change though...
impl<'a, T, N> Readable<'a> for FixedArray<T, N>
    where N: ArrayLength<T>,
          T: Readable<'a> + Default,
          T::Args: Clone,
{
    type Args = T::Args;


    #[inline]
    fn read(mut reader: Reader<'a>, args: Self::Args) -> (Self, Reader<'a>)
    {
        let mut array = GenericArray::new();
        for i in 0..N::to_usize() {
            array[i] = reader.read(args.clone());
        }
        (array, reader)
    }

    #[inline]
    fn size(&self) -> usize
    {
        <Self as Readable>::fixed_size()
            .unwrap_or_else(|| self.iter().fold(0, |s, i| s + i.size()))
    }


    #[inline]
    fn fixed_size() -> Option<usize>
    {
        T::fixed_size().map(|i| i * N::to_usize())
    }
}

impl<'a, T, N> Writable for FixedArray<T, N>
    where N: ArrayLength<T>,
          T: Readable<'a> + Default + Writable,
          T::Args: Clone,
{
    fn write<W: io::Write>(&self, writer: &mut W) -> io::Result<()>
    {
        for elem in self.iter() {
            elem.write(writer)?
        }
        Ok(())
    }
}
