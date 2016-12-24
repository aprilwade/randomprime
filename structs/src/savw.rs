use reader_writer::{LazyArray, RoArray, pad_bytes_count, pad_bytes_ff};

auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Clone, Debug)]
    pub struct Savw<'a>
    {
        #[expect = 0xC001D00D]
        magic: u32,

        #[expect = 3]
        version: u32,

        area_count: u32,

        #[derivable = cinematic_skip_array.len() as u32]
        cinematic_skip_count: u32,
        cinematic_skip_array: RoArray<'a, u32> = (cinematic_skip_count as usize, ()),

        #[derivable = memory_relay_array.len() as u32]
        memory_relay_count: u32,
        memory_relay_array: RoArray<'a, u32> = (memory_relay_count as usize, ()),

        #[derivable = layer_toggle_array.len() as u32]
        layer_toggle_count: u32,
        layer_toggle_array: RoArray<'a, LayerToggle> = (layer_toggle_count as usize, ()),

        #[derivable = door_array.len() as u32]
        door_count: u32,
        door_array: RoArray<'a, u32> = (door_count as usize, ()),

        #[derivable = scan_array.len() as u32]
        scan_count: u32,
        scan_array: LazyArray<'a, ScannableObject> = (scan_count as usize, ()),

        #[offset]
        offset_after: usize,
        #[derivable = pad_bytes_ff(32, offset_after)]
        _padding_after: RoArray<'a, u8> = (pad_bytes_count(32, offset_after), ()),
    }
}

auto_struct! {
    #[auto_struct(Readable, Writable, FixedSize)]
    #[derive(Clone, Debug)]
    pub struct LayerToggle
    {
        area_id: u32,
        layer_index: u32,
    }
}

auto_struct! {
    #[auto_struct(Readable, Writable, FixedSize)]
    #[derive(Clone, Debug)]
    pub struct ScannableObject
    {
        scan: u32,
        logbook_category: u32,
    }
}

