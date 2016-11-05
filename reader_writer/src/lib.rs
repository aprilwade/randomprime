extern crate byteorder;
pub extern crate generic_array;
extern crate num;

mod reader;
mod writer;

mod primitive_types;
mod lazy;
mod fixed_array;
mod array;
mod lengths_array;
mod for_each_array;
mod iterator_array;

mod imm_cow;
mod ref_iterable;
mod derivable_array_proxy;

mod cursor;

mod padding;

pub use generic_array::typenum;

pub use reader::{Reader, Readable};
pub use writer::Writable;

pub use primitive_types::{FourCC, CStr};
pub use lazy::{Lazy, LazySized};
pub use fixed_array::FixedArray;
pub use array::{Array, ArrayIterator, ArrayBorrowedIterator};
pub use lengths_array::{LengthsArray, LengthsArrayIterator, LengthsArrayLengthsIterator};
//pub use for_each_array::{ForEachArray, ForEachArrayIterator};
pub use iterator_array::{IteratorArray, IteratorArrayIterator};
pub use derivable_array_proxy::Dap;

pub use imm_cow::ImmCow;

pub use padding::*;
