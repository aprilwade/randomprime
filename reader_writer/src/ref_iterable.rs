use std::slice::Iter as SliceIter;
use std::borrow::Borrow;

pub trait RefIterable<'a, T: 'a>
{
    type Item: Borrow<T> + 'a;
    type Iter: Iterator<Item=Self::Item> + ExactSizeIterator;
    fn ref_iter(&'a self) -> Self::Iter;
    //fn len(&self) -> usize;
}

/*
impl<'a, T: 'a, I: 'a, B: 'a> RefIterable<'a, T> for I
    where &'a I: IntoIterator<Item=B>,
          B: Borrow<T>,
{
    type Item = B;
    type Iter = <&'a I as IntoIterator>::IntoIter;
    fn ref_iter(&'a self) -> Self::Iter
    {
        self.into_iter()
    }
}
*/

impl<'a, T: 'a> RefIterable<'a, T> for Vec<T>
{
    type Item = &'a T;
    type Iter = SliceIter<'a, T>;
    fn ref_iter(&'a self) -> Self::Iter
    {
        self.iter()
    }

    /*fn len(&self) -> usize
    {
        Vec::len(self)
    }*/
}

/*
struct Struct<I, L>
    where for<'a> L: RefIterable<'a, I>,
{
    l: L,
    i: ::std::marker::PhantomData<I>,
}

fn test()
{
    let s: Struct<_, Vec<u32>> = Struct { l: vec![1, 2, 3], i: ::std::marker::PhantomData };
}
*/
