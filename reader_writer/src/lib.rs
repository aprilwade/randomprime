pub extern crate byteorder;
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


pub use crate::generic_array::typenum;

pub use crate::reader::{Reader, Readable};
pub use crate::writer::Writable;

pub use crate::primitive_types::{FourCC, CStr, CStrConversionExtension};
pub use crate::array::{LazyArray, LazyArrayIter};
pub use crate::read_only_array::{RoArray, RoArrayIter};
pub use crate::fixed_array::FixedArray;
pub use crate::iterator_array::{IteratorArray, IteratorArrayIterator};
pub use crate::derivable_array_proxy::Dap;
pub use crate::uncached::Uncached;

pub use crate::lcow::LCow;

// XXX There are > 5 items in these modules. Do I want to use * imports everywhere for
//     consistency?
pub use crate::padding::*;
pub use crate::diff_list::*;
pub use crate::utf16_string:: *;
