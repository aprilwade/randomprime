
use SclyPropertyData;
use reader_writer::CStr;

auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Debug, Clone)]
    pub struct HudMemo<'a>
    {
        #[expect = 6]
        prop_count: u32,

        name: CStr<'a>,

        first_message_timer: f32,
        unknown: u8,
        memo_type: u32,
        strg: u32,
        active: u8,
    }
}

impl<'a> SclyPropertyData for HudMemo<'a>
{
    const OBJECT_TYPE: u8 = 0x17;
}
