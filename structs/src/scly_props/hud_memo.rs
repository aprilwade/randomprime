
use reader_writer::CStr;

auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Debug, Clone)]
    pub struct HudMemo<'a>
    {
        // 6 properties
        name: CStr<'a>,

        first_message_timer: f32,
        unknown: u8,
        memo_type: u32,
        strg: u32,
        active: u8,
    }
}
