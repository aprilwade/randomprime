
use SclyPropertyData;
use reader_writer::CStr;

auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Debug, Clone)]
    pub struct Relay<'a>
    {
        #[expect = 2]
        prop_count: u32,

        name: CStr<'a>,

        active: u8,
    }
}

impl<'a> SclyPropertyData for Relay<'a>
{
    const OBJECT_TYPE: u8 = 0x15;
}
