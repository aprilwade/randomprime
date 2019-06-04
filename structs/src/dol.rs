use auto_struct_macros::auto_struct;


use std::iter::Zip as ZipIter;
use reader_writer::{IteratorArray, LCow, LazyArray};
use reader_writer::generic_array::{GenericArray, GenericArrayIter};
use reader_writer::typenum::*;


pub type DolSegementsIter<S> = ZipIter<GenericArrayIter<u32, S>, GenericArrayIter<u32, S>>;
#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
// XXX We're assuming that all of the segments are contigious and in order, which isn't
//     necessarily the case but is true for prime
pub struct Dol<'r>
{
    #[auto_struct(derive_from_iter = text_segments.iter()
        .scan(0x100, &|sum: &mut usize, seg: LCow<DolSegment>| {
            let r = *sum as u32;
            *sum += seg.contents.len();
            Some(r)
        }))]
    _text_offsets: GenericArray<u32, U7>,
    #[auto_struct(derive_from_iter = text_segments.iter()
        .scan(
            0x100 + text_segments.iter().map(|seg| seg.contents.len()).sum::<usize>(),
            &|sum: &mut usize, seg: LCow<DolSegment>| {
                let r = *sum as u32;
                *sum += seg.contents.len();
                Some(r)
            }
        ))]
    _data_offsets: GenericArray<u32, U11>,
    #[auto_struct(derive_from_iter = text_segments.iter().map(|s| s.load_addr))]
    text_load_addrs: GenericArray<u32, U7>,
    #[auto_struct(derive_from_iter = data_segments.iter().map(|s| s.load_addr))]
    data_load_addrs: GenericArray<u32, U11>,
    #[auto_struct(derive_from_iter = text_segments.iter().map(|s| s.contents.len() as u32))]
    text_sizes: GenericArray<u32, U7>,
    #[auto_struct(derive_from_iter = data_segments.iter().map(|s| s.contents.len() as u32))]
    data_sizes: GenericArray<u32, U11>,
    pub bss_addr: u32,
    pub bss_size: u32,
    pub entry_point: u32,

    #[auto_struct(expect = [0u8; 28].into())]
    _padding: GenericArray<u8, U28>,

    #[auto_struct(init = text_load_addrs.into_iter().zip(text_sizes.into_iter()))]
    pub text_segments: IteratorArray<'r, DolSegment<'r>, DolSegementsIter<U7>>,
    #[auto_struct(init = data_load_addrs.into_iter().zip(data_sizes.into_iter()))]
    pub data_segments: IteratorArray<'r, DolSegment<'r>, DolSegementsIter<U11>>,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct DolSegment<'r>
{
    #[auto_struct(args = (load_addr, size))]
    _args: (u32, u32),

    #[auto_struct(literal = load_addr)]
    pub load_addr: u32,
    #[auto_struct(init = (size as usize, ()))]
    pub contents: LazyArray<'r, u8>,
}
