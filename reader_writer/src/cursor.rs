use reader::{Reader, Readable};

pub trait RandomAccess
{
    type Item;
    fn get(&self, at: usize) -> Option<Self::Item>;
    fn len(&self) -> usize;
}

pub trait RandomAccessRef<'a>
{
    type Item: 'a;
    fn get(&'a self, at: usize) -> Option<Self::Item>;
    fn len(&self) -> usize;
}

impl<T> RandomAccess for Vec<T>
    where T: Clone,
{
    type Item = T;
    fn get(&self, at: usize) -> Option<Self::Item>
    {
        (&**self).get(at).cloned()
    }

    fn len(&self) -> usize
    {
        (&**self).len()
    }
}

struct FEA<'a, T, F>
    where T: Readable<'a>,
          T::Args: Clone,
          F: RandomAccess<Item=T::Args>,
{
    from: F,
    reader: Reader<'a>,
    pd: ::std::marker::PhantomData<*const T>,
}

impl<'a, T, F> Readable<'a> for FEA<'a, T, F>
    where T: Readable<'a>,
          T::Args: Clone,
          F: RandomAccess<Item=T::Args>,
{
    type Args = F;
    fn read(reader: Reader<'a>, from: Self::Args) -> (Self, Reader<'a>)
    {
        let fea = FEA {
            from: from,
            reader: reader.clone(),
            pd: ::std::marker::PhantomData,
        };
        let s = fea.size();
        (fea, reader.offset(s))
    }

    fn size(&self) -> usize
    {
        let mut sum = 0;
        let mut reader = self.reader.clone();
        for i in 0..self.from.len() {
            sum += reader.read::<T>(self.from.get(i).unwrap()).size();
        }
        sum
    }
}
