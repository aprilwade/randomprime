use std::borrow::Borrow;
use std::marker::PhantomData;
use std::slice::Iter as SliceIter;
use std::slice::IterMut as SliceIterMut;
use std::io::Write;

use num::ToPrimitive;

use reader::{Reader, Readable};
use writer::Writable;
use ref_iterable::RefIterable;
use imm_cow::ImmCow;


#[derive(Clone)]
pub enum LengthsArray<'a, T, L, I>
    where T: Readable<'a>,
          T::Args: Clone,
          for<'t> &'t T: Into<usize>,
          I: ToPrimitive,
          for<'r> L: RefIterable<'r, I>,
          L: Clone,
{
    Borrowed(Reader<'a>, L, T::Args, PhantomData<*const I>),
    Owned(Vec<T>),
}

impl<'a, T, L, I> LengthsArray<'a, T, L, I>
    where T: Readable<'a>,
          T::Args: Clone,
          for<'t> &'t T: Into<usize>,
          I: ToPrimitive,
          for<'r> L: RefIterable<'r, I>,
          L: Clone,
{
    pub fn len(&self) -> usize
    {
        match *self {
            LengthsArray::Borrowed(_, ref lengths, _, _) => lengths.ref_iter().len(),
            LengthsArray::Owned(ref vec) => vec.len(),
        }
    }

    pub fn iter<'s>(&'s self) -> LengthsArrayIterator<'s, 'a, T, L, I>
    {
        self.into_iter()
    }
    /*
    // XXX: If <&LengthsArray as IntoIterator> doesn't work, try this instead?
    pub fn iter<'s>(&'s self) -> LengthsArrayIterator<'s, 'a, T, L, I>
    {
        match *self {
            LengthsArray::Borrowed(ref reader, ref lengths, ref args, _) => {
                LengthsArrayIterator::Borrowed(
                    reader.clone(),
                    lengths.ref_iter(),
                    args,
                    PhantomData
                )
            },
            LengthsArray::Owned(ref vec) => LengthsArrayIterator::Owned(vec.iter()),
        }
    }
    */

    #[inline]
    pub fn iter_mut<'s>(&'s mut self) -> SliceIterMut<'s, T>
    {
        self.as_mut_vec().iter_mut()
    }

    pub fn as_mut_vec(&mut self) -> &mut Vec<T>
    {
        *self = match *self {
            LengthsArray::Borrowed(ref mut reader, ref lengths, ref args, _) => {
                let lengths_iter = lengths.ref_iter();
                let mut vec = Vec::with_capacity(lengths_iter.len());
                for length in lengths_iter {
                    let res = reader.clone().read::<T>(args.clone());
                    let length = length.borrow().to_usize().unwrap();
                    vec.push(res);
                    reader.advance(length);
                };
                LengthsArray::Owned(vec)
            },
            LengthsArray::Owned(ref mut vec) => return vec,
        };
        match *self {
            LengthsArray::Owned(ref mut vec) => vec,
            LengthsArray::Borrowed(_, _, _, _) => unreachable!(),
        }
    }

    pub fn lengths_iter<'s>(&'s self) -> LengthsArrayLengthsIterator<'s, 'a, T, L, I>
    {
        match *self {
            LengthsArray::Owned(ref vec) => LengthsArrayLengthsIterator::Owned(vec.iter()),
            LengthsArray::Borrowed(_, ref l, _, _) =>
                LengthsArrayLengthsIterator::Borrowed(l.ref_iter(), PhantomData),
        }
    }
}

pub enum LengthsArrayIterator<'s, 'a: 's, T, L, I>
    where T: Readable<'a> + 's,
          T::Args: Clone,
          for<'t> &'t T: Into<usize>,
          I: ToPrimitive + 's,
          L: RefIterable<'s, I>,
          L: Clone,
{
    Borrowed(Reader<'a>, L::Iter, &'s T::Args, PhantomData<*const I>),
    Owned(SliceIter<'s, T>),
}

impl<'s, 'a: 's, T, L, I> Iterator for LengthsArrayIterator<'s, 'a, T, L, I>
    where T: Readable<'a> + 's,
          T::Args: Clone,
          for<'t> &'t T: Into<usize>,
          I: ToPrimitive + 's,
          L: RefIterable<'s, I>,
          L: Clone,
{
    type Item = ImmCow<'s, T>;
    fn next(&mut self) -> Option<Self::Item>
    {
        match *self {
            LengthsArrayIterator::Borrowed(ref mut reader, ref mut lengths_iter, args, _) => {
                if let Some(length) = lengths_iter.next() {
                    let res = reader.clone().read::<T>(args.clone());
                    let length = length.borrow().to_usize().unwrap();
                    reader.advance(length);
                    Some(ImmCow::new_owned(res))
                } else {
                    None
                }
            },
            LengthsArrayIterator::Owned(ref mut iter) => iter.next().map(ImmCow::new_borrowed),
        }
    }
}

pub enum LengthsArrayLengthsIterator<'s, 'a: 's, T, L, I>
    where T: Readable<'a> + 's,
          T::Args: Clone,
          for<'t> &'t T: Into<usize>,
          I: ToPrimitive + 's,
          L: RefIterable<'s, I>,
          L: Clone,
{
    Borrowed(L::Iter, PhantomData<*const (&'a (), I)>),
    Owned(SliceIter<'s, T>),
}


impl<'s, 'a: 's, T, L, I> Iterator for LengthsArrayLengthsIterator<'s, 'a, T, L, I>
    where T: Readable<'a> + 's,
          T::Args: Clone,
          for<'t> &'t T: Into<usize>,
          I: ToPrimitive + 's,
          L: RefIterable<'s, I>,
          L: Clone,
{
    type Item = usize;
    fn next(&mut self) -> Option<Self::Item>
    {
        match *self {
            LengthsArrayLengthsIterator::Borrowed(ref mut iter, _) =>
                    iter.next().map(|i| i.borrow().to_usize().unwrap()),
            LengthsArrayLengthsIterator::Owned(ref mut iter) => iter.next().map(|t| t.into()),
        }
    }
}

impl<'s, 'a: 's, T, L, I> IntoIterator for &'s LengthsArray<'a, T, L, I>
    where T: Readable<'a> + 's,
          T::Args: Clone,
          for<'t> &'t T: Into<usize>,
          I: ToPrimitive + 's,
          for<'r> L: RefIterable<'r, I>,
          L: Clone,
{
    type Item = <LengthsArrayIterator<'s, 'a, T, L, I> as Iterator>::Item;
    type IntoIter = LengthsArrayIterator<'s, 'a, T, L, I>;
    fn into_iter(self) -> Self::IntoIter
    {
        match *self {
            LengthsArray::Borrowed(ref reader, ref lengths, ref args, _) => {
                LengthsArrayIterator::Borrowed(
                    reader.clone(),
                    lengths.ref_iter(),
                    args,
                    PhantomData
                )
            },
            LengthsArray::Owned(ref vec) => LengthsArrayIterator::Owned(vec.iter()),
        }
    }
}

impl<'a, T, L, I> Readable<'a> for LengthsArray<'a, T, L, I>
    where T: Readable<'a> + Writable,
          T::Args: Clone,
          for<'t> &'t T: Into<usize>,
          I: ToPrimitive,
          for<'r> L: RefIterable<'r, I>,
          L: Clone,
{
    type Args = (L, T::Args);
    #[inline]
    fn read(mut reader: Reader<'a>, (lengths, args): Self::Args) -> (Self, Reader<'a>)
    {
        let la = LengthsArray::Borrowed(reader.clone(), lengths, args, PhantomData);
        reader.advance(la.size());
        (la, reader)
    }

    #[inline]
    fn size(&self) -> usize
    {
        match *self {
            LengthsArray::Borrowed(_, ref lengths, _, _) => {
                lengths.ref_iter().fold(0, |l, r| l + r.borrow().to_usize().unwrap())
            },
            LengthsArray::Owned(ref vec) => {
                vec.iter().fold(0, |l, r| l + <&T as Into<usize>>::into(r))
            }
        }
    }
}

impl<'a, T, L, I> Writable for LengthsArray<'a, T, L, I>
    where T: Readable<'a> + Writable,
          T::Args: Clone,
          for<'t> &'t T: Into<usize>,
          I: ToPrimitive,
          for<'r> L: RefIterable<'r, I>,
          L: Clone,
{
    fn write<W: Write>(&self, writer: &mut W)
    {
        match *self {
            LengthsArray::Borrowed(ref reader, _, _, _) => {
                // Just copy the bytes directly
                let len = self.size();
                writer.write(&(*reader)[0..len]).unwrap();
            },
            LengthsArray::Owned(ref vec) => {
                for elem in vec {
                    // TODO: Is this right? Do I need to manually reposition write?
                    elem.write(writer);
                }
            },
        }
    }
}
/*
fn test()
{
    use ::array::Array;
    let n = 10;
    let data = &[1, 2, 3];
    let la: LengthsArray<u32, Array<u32>, u32> = LengthsArray::Borrowed(
        Reader::new(&data[..]),
        Reader::new(&data[..]).read((3, ())),
        (),
        PhantomData
    );
    //let la: LengthsArray<u32, Vec<u32>, u32> =
    //    LengthsArray::Borrowed(Reader::new(&data[..]), vec![1, 2, 3], (), PhantomData);
}
*/
