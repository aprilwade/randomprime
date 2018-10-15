
use SclyPropertyData;
use reader_writer::CStr;


auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Debug, Clone)]
    pub struct MemoryRelay<'a>
    {
        #[expect = 3]
        prop_count: u32,

        name: CStr<'a>,
        unknown: u8,
        active: u8,
    }
}

impl<'a> SclyPropertyData for MemoryRelay<'a>
{
    const OBJECT_TYPE: u8 = 0x13;
}
