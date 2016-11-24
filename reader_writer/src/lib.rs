extern crate byteorder;
pub extern crate generic_array;
extern crate num;
extern crate linked_list;

mod reader;
mod writer;

mod primitive_types;
mod lazy;
mod fixed_array;
mod array;
mod read_only_array;
mod iterator_array;
mod diff_list;

mod imm_cow;
mod derivable_array_proxy;
mod uncached;

mod padding;


pub use generic_array::typenum;

pub use reader::{Reader, Readable};
pub use writer::Writable;

pub use primitive_types::{FourCC, CStr};
pub use lazy::{Lazy, LazySized};
pub use array::{LazyArray, LazyArrayIter};
pub use read_only_array::{RoArray, RoArrayIter};
pub use fixed_array::FixedArray;
pub use iterator_array::{IteratorArray, IteratorArrayIterator};
pub use derivable_array_proxy::Dap;
pub use uncached::Uncached;

pub use imm_cow::ImmCow;

// XXX There are > 5 items in these modules. Do I want to use * imports everywhere for
//     consistency?
pub use padding::*;
pub use diff_list::*;
