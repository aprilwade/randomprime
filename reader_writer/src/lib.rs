extern crate byteorder;
pub extern crate generic_array;
pub extern crate num;

mod reader;
mod writer;

mod primitive_types;
mod fixed_array;
mod array;
mod read_only_array;
mod iterator_array;
mod diff_list;

mod lcow;
mod derivable_array_proxy;
mod uncached;

mod padding;

mod utf16_string;


pub use generic_array::typenum;

pub use reader::{Reader, Readable};
pub use writer::Writable;

pub use primitive_types::{FourCC, CStr};
pub use array::{LazyArray, LazyArrayIter};
pub use read_only_array::{RoArray, RoArrayIter};
pub use fixed_array::FixedArray;
pub use iterator_array::{IteratorArray, IteratorArrayIterator};
pub use derivable_array_proxy::Dap;
pub use uncached::Uncached;

pub use lcow::LCow;

// XXX There are > 5 items in these modules. Do I want to use * imports everywhere for
//     consistency?
pub use padding::*;
pub use diff_list::*;
pub use utf16_string:: *;
