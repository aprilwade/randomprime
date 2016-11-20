
use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;

auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Debug, Clone)]
    pub struct StreamedAudio<'a>
    {
        // 9 properties
        name: CStr<'a>,

        active: u8,
        audio_file_name: CStr<'a>,

        // 6 unknown properties
        unknowns: GenericArray<u8, U18>,
    }
}
