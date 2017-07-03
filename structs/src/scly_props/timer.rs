
use SclyPropertyData;
use reader_writer::CStr;

auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Debug, Clone)]
    pub struct Timer<'a>
    {
        #[expect = 6]
        prop_count: u32,

        name: CStr<'a>,

        start_time: f32,
        max_random_add: f32,
        reset_to_zero: u8,
        start_immediately: u8,
        active: u8,
    }
}

impl<'a> SclyPropertyData for Timer<'a>
{
    fn object_type() -> u8
    {
        0x5
    }
}
