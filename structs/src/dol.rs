

use std::iter::Zip as ZipIter;
use reader_writer::{Dap, IteratorArray, LCow, LazyArray};
use reader_writer::generic_array::{GenericArray, GenericArrayIter};
use reader_writer::typenum::*;


pub type DolSegementsIter<S> = ZipIter<GenericArrayIter<u32, S>, GenericArrayIter<u32, S>>;
auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Debug, Clone)]
    // XXX We're assuming that all of the segments are contigious and in order, which isn't
    //     necessarily the case but is true for prime
    pub struct Dol<'a>
    {
        #[derivable: Dap<_, _> = text_segments.iter()
            .scan(0x100, &|sum: &mut usize, seg: LCow<DolSegment>| {
                let r = *sum as u32;
                *sum += seg.contents.len();
                Some(r)
            }).into()]
        _text_offsets: GenericArray<u32, U7>,
        #[derivable: Dap<_, _> = text_segments.iter()
            .scan(
                0x100 + text_segments.iter().map(|seg| seg.contents.len()).sum::<usize>(),
                &|sum: &mut usize, seg: LCow<DolSegment>| {
                    let r = *sum as u32;
                    *sum += seg.contents.len();
                    Some(r)
                }
            ).into()]
        _data_offsets: GenericArray<u32, U11>,
        #[derivable: Dap<_, _> = text_segments.iter().map(|s| s.load_addr).into()]
        text_load_addrs: GenericArray<u32, U7>,
        #[derivable: Dap<_, _> = data_segments.iter().map(|s| s.load_addr).into()]
        data_load_addrs: GenericArray<u32, U11>,
        #[derivable: Dap<_, _> = text_segments.iter().map(|s| s.contents.len() as u32).into()]
        text_sizes: GenericArray<u32, U7>,
        #[derivable: Dap<_, _> = data_segments.iter().map(|s| s.contents.len() as u32).into()]
        data_sizes: GenericArray<u32, U11>,
        bss_addr: u32,
        bss_size: u32,
        entry_point: u32,

        #[derivable = [0u8; 28].into()]
        _padding: GenericArray<u8, U28>,

        text_segments: IteratorArray<'a, DolSegment<'a>, DolSegementsIter<U7>> =
            text_load_addrs.into_iter().zip(text_sizes.into_iter()),
        data_segments: IteratorArray<'a, DolSegment<'a>, DolSegementsIter<U11>> =
            data_load_addrs.into_iter().zip(data_sizes.into_iter()),
    }
}

auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Debug, Clone)]
    pub struct DolSegment<'a>
    {
        #[args]
        (load_addr, size): (u32, u32),

        #[literal]
        load_addr: u32 = load_addr,
        contents: LazyArray<'a, u8> = (size as usize, ()),
    }
}
