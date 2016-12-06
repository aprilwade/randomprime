
// This is intentionally _very_ incomplete. The particle format is huge, so only some of
// the substructures are implemented.


use reader_writer::RoArray;



auto_struct! {
    #[auto_struct(Readable)]
    #[derive(Debug, Clone)]
    pub struct Kssm<'a>
    {
        unknown0: u32,
        unknown1: u32,
        end_frame: u32,
        unknown2: u32,
        list_count: u32,
        lists: RoArray<'a, KssmFrameInfo<'a>> = (list_count as usize, ()),
    }
}

auto_struct! {
    #[auto_struct(Readable)]
    #[derive(Debug, Clone)]
    pub struct KssmFrameInfo<'a>
    {
        frame: u32,
        item_count: u32,
        items: RoArray<'a, KssmFrameInfoItem> = (item_count as usize, ()),
    }
}

auto_struct! {
    #[auto_struct(Readable)]
    #[derive(Debug, Clone)]
    pub struct KssmFrameInfoItem
    {
        part: u32,
        unknown0: u32,
        unknown1: u32,
        unknown2: u32,
    }
}
