// Rexport these crates to make syncing version numbers less of a pain
pub use byteorder;
pub use generic_array;

pub mod reader;
pub mod writer;

pub mod primitive_types;
pub mod fixed_array;
pub mod array;
pub mod read_only_array;
pub mod iterator_array;

pub mod lcow;
pub mod derivable_array_proxy;
pub mod uncached;
pub mod with_read;

pub mod padding;

pub mod utf16_string;


pub use crate::{
    generic_array::typenum,

    reader::{Reader, Readable},
    writer::Writable,

    primitive_types::{FourCC, CStr, CStrConversionExtension},
    array::{LazyArray, LazyArrayIter},
    read_only_array::{RoArray, RoArrayIter},
    fixed_array::FixedArray,
    iterator_array::{IteratorArray, IteratorArrayIterator},
    derivable_array_proxy::{Dap, DerivableFromIterator},
    uncached::Uncached,
    with_read::WithRead,

    lcow::LCow,

    // XXX There are > 5 items in these modules. Do I want to use * imports everywhere for
    //     consistency?
    padding::*,
    utf16_string:: *,
};
