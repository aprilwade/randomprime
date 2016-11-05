use std::marker::PhantomData;
use std::borrow::Borrow;
use std::slice::Iter as SliceIter;

use reader::{Reader, Readable};
use imm_cow::ImmCow;
use ref_iterable::RefIterable;
    

/*
trait ForEachAdaptor

{
    type Output;
    fn 
}
*/



pub enum ForEachArray<'a, F, T, A, C>
    where T: Readable<'a>,
          for<'r> A: RefIterable<'r, F>,
          A: Clone,
          C: Borrow<Fn(&F) -> T::Args>,
{
    Borrowed(Reader<'a>, A, C, PhantomData<*const F>),
    Owned(Vec<T>),
}


impl<'a, F, T, A, C> ForEachArray<'a, F, T, A, C>
    where T: Readable<'a>,
          for<'r> A: RefIterable<'r, F>,
          A: Clone,
          C: Borrow<Fn(&F) -> T::Args>,
{
    pub fn len(&self) -> usize
    {
        match *self {
            ForEachArray::Borrowed(_, ref a, _, _) => a.ref_iter().len(),
            ForEachArray::Owned(ref vec) => vec.len(),
        }
    }

    pub fn iter<'s>(&'s self) -> ForEachArrayIterator<'s, 'a, F, T, A, C>
    {
        self.into_iter()
    }

    pub fn from_iter<'s>(&'s self) -> ForEachArrayFromIterator<'s, F, T, A>
    where F: Clone + 's,
          for<'r> &'r T: Into<F>
    {
        match *self {
            ForEachArray::Borrowed(_, ref a, _, _) =>
                ForEachArrayFromIterator::Borrowed(a.ref_iter()),
            ForEachArray::Owned(ref vec) => ForEachArrayFromIterator::Owned(vec.iter()),
        }
    }
}

impl<'s, 'a: 's, F, T, A, C> IntoIterator for &'s ForEachArray<'a, F, T, A, C>
where T: Readable<'a>,
      for<'r> A: RefIterable<'r, F>,
      A: Clone,
      C: Borrow<Fn(&F) -> T::Args>,
{
    type Item = <ForEachArrayIterator<'s, 'a, F, T, A, C> as Iterator>::Item;
    type IntoIter = ForEachArrayIterator<'s, 'a, F, T, A, C>;
    fn into_iter(self) -> Self::IntoIter
    {
        match *self {
            ForEachArray::Borrowed(ref reader, ref from_array, ref ctor, _) => {
                ForEachArrayIterator::Borrowed(
                    reader.clone(),
                    from_array.ref_iter(),
                    ctor
                )
            },
            ForEachArray::Owned(ref vec) => ForEachArrayIterator::Owned(vec.iter()),
        }
    }
}


pub enum ForEachArrayIterator<'s, 'a: 's, F, T, A, C>
    where T: Readable<'a> + 's,
          A: RefIterable<'s, F>,
          F: 's,
          A: Clone,
          C: Borrow<Fn(&F) -> T::Args> + 's,
{
    Borrowed(Reader<'a>, A::Iter, &'s C),
    Owned(SliceIter<'s, T>),
}

impl<'s, 'a: 's, F, T, A, C> Iterator for ForEachArrayIterator<'s, 'a, F, T, A, C>
    where T: Readable<'a> + 's,
          A: RefIterable<'s, F>,
          F: 's,
          A: Clone,
          C: Borrow<Fn(&F) -> T::Args> + 's,
{
    type Item = ImmCow<'s, T>;
    fn next(&mut self) -> Option<Self::Item>
    {
        match *self {
            ForEachArrayIterator::Borrowed(ref mut reader, ref mut from_iter, ctor) => {
                if let Some(from) = from_iter.next() {
                    let res = reader.read::<T>(ctor.borrow()(from.borrow()));
                    Some(ImmCow::new_owned(res))
                } else {
                    None
                }
            },
            ForEachArrayIterator::Owned(ref mut iter) => iter.next().map(ImmCow::new_borrowed),
        }
    }
}

pub enum ForEachArrayFromIterator<'s, F, T, A>
    where T: 's,
          A: RefIterable<'s, F>,
          F: Clone + 's,
          for<'r> &'r T: Into<F>
{
    Borrowed(A::Iter),
    Owned(SliceIter<'s, T>),
}

impl<'s, F, T, A> Iterator for ForEachArrayFromIterator<'s, F, T, A>
    where T: 's,
          A: RefIterable<'s, F>,
          F: Clone + 's,
          for<'r> &'r T: Into<F>
{
    type Item = F;
    fn next(&mut self) -> Option<Self::Item>
    {
        match *self {
            ForEachArrayFromIterator::Borrowed(ref mut from_iter) =>
                from_iter.next().map(|i| (*i.borrow()).clone()),
            ForEachArrayFromIterator::Owned(ref mut iter) =>
                iter.next().map(|i| <&T as Into<F>>::into(i)),
        }
    }
}



impl<'a, F, T, A, C> Readable<'a> for ForEachArray<'a, F, T, A, C>
    where T: Readable<'a>,
          for<'r> A: RefIterable<'r, F>,
          A: Clone,
          C: Borrow<Fn(&F) -> T::Args>,
{
    type Args = (A, C);
    fn read(mut reader: Reader<'a>, (from_array, ctor): Self::Args) -> (Self, Reader<'a>)
    {
        let res = ForEachArray::Borrowed(reader.clone(), from_array, ctor, PhantomData);
        reader.advance(res.size());
        (res, reader)
    }

    fn size(&self) -> usize
    {
        if let Some(i) = T::fixed_size() {
            i * self.len()
        } else {
            self.iter().map(|i| i.size()).sum()
        }
    }
}
