use reader_writer::{CStr, LazyArray};

auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Debug, Clone)]
    pub struct Hint<'a>
    {
        #[expect = 0x00BADBAD]
        magic: u32,
        #[expect = 1]
        version: u32,

        #[derivable = hints.len() as u32]
        hint_count: u32,
        hints: LazyArray<'a, HintDetails<'a>> = (hint_count as usize, ()),
    }
}

auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Debug, Clone)]
    pub struct HintDetails<'a>
    {
        hint_name: CStr<'a>,
        intermediate_time: f32,
        normal_time: f32,
        popup_text_strg: u32,
        text_time: u32,

        #[derivable = locations.len() as u32]
        location_count: u32,
        locations: LazyArray<'a, HintLocation> = (location_count as usize, ()),
    }
}

auto_struct! {
    #[auto_struct(Readable, Writable, FixedSize)]
    #[derive(Debug, Clone)]
    pub struct HintLocation
    {
        mlvl: u32,
        mrea: u32,
        target_room_index: u32,
        map_text_strg: u32,
    }
}
