
use SclyPropertyData;
use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;

auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Debug, Clone)]
    pub struct StreamedAudio<'a>
    {
        #[expect = 9]
        prop_count: u32,

        name: CStr<'a>,

        active: u8,
        audio_file_name: CStr<'a>,

        // 6 unknown properties
        unknowns: GenericArray<u8, U18>,
    }
}

impl<'a> SclyPropertyData for StreamedAudio<'a>
{
    const OBJECT_TYPE: u8 = 0x61;
}
