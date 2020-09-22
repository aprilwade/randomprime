use auto_struct_macros::auto_struct;

// This is intentionally _very_ incomplete. The particle format is huge, so only some of
// the substructures are implemented.

use reader_writer::RoArray;

use crate::ResId;
use crate::res_id::*;

#[auto_struct(Readable)]
#[derive(Debug, Clone)]
pub struct Kssm<'r>
{
    pub unknown0: u32,
    pub unknown1: u32,
    pub end_frame: u32,
    pub unknown2: u32,
    pub list_count: u32,
    #[auto_struct(init = (list_count as usize, ()))]
    pub lists: RoArray<'r, KssmFrameInfo<'r>>,
}

#[auto_struct(Readable)]
#[derive(Debug, Clone)]
pub struct KssmFrameInfo<'r>
{
    pub frame: u32,
    pub item_count: u32,
    #[auto_struct(init = (item_count as usize, ()))]
    pub items: RoArray<'r, KssmFrameInfoItem>,
}

#[auto_struct(Readable)]
#[derive(Debug, Clone)]
pub struct KssmFrameInfoItem
{
    pub part: ResId<PART>,
    pub unknown0: u32,
    pub unknown1: u32,
    pub unknown2: u32,
}
