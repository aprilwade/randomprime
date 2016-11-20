
use reader_writer::CStr;

auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Debug, Clone)]
    pub struct Relay<'a>
    {
        // 2 properties
        name: CStr<'a>,

        active: u8,
    }
}
