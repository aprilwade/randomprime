use auto_struct_macros::auto_struct;

use reader_writer::CStr;
use crate::SclyPropertyData;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct StreamedAudio<'r>
{
    #[auto_struct(expect = 9)]
    prop_count: u32,

    pub name: CStr<'r>,

    pub active: u8,
    pub audio_file_name: CStr<'r>,

    pub no_stop_on_deactivate: u8,
    pub fade_in_time: f32,
    pub fade_out_time: f32,
    pub volume: u32,
    pub oneshot: u32,
    pub is_music: u8,
}

impl<'r> SclyPropertyData for StreamedAudio<'r>
{
    const OBJECT_TYPE: u8 = 0x61;
}
