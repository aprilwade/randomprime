use auto_struct_macros::auto_struct;

use reader_writer::{FourCC, IteratorArray, LazyArray, Readable, RoArray, RoArrayIter};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Thp<'r>
{
    #[auto_struct(expect = b"THP\0".into())]
    magic: FourCC,

    #[auto_struct(expect = 0x00010000)]
    version: u32,

    pub max_buffer_size: u32,
    pub max_audio_samples: u32,

    #[auto_struct(expect = 0x41efc28f)]
    fps: u32,
    #[auto_struct(derive = frames.len() as u32)]
    frame_count: u32,
    #[auto_struct(derive = frames.iter().next().unwrap().size() as u32)]
    _first_frame_size: u32,
    #[auto_struct(derive = frames.size() as u32)]
    _data_size: u32,

    #[auto_struct(expect = 0x30)]
    _component_data_offset: u32,
    #[auto_struct(expect = 0)]
    _offsets_data_offset: u32,
    #[auto_struct(derive = 0x30 + components.size() as u32)]
    _first_frame_offset: u32,
    #[auto_struct(derive = (0x30 + components.size() + frames.size() -
                    frames.iter().last().unwrap().size()) as u32)]
    _last_frame_offset: u32,

    pub components: ThpComponents<'r>,
    #[auto_struct(init = (frame_count as usize, components.component_count > 1))]
    pub frames: LazyArray<'r, ThpFrameData<'r>>,
}

impl<'r> Thp<'r>
{
    pub fn update_sibling_frame_sizes(&mut self)
    {
        if !self.frames.is_owned() {
            return;
        }
        let vec = self.frames.as_mut_vec();
        let first_size = vec.first().unwrap().size() as u32;
        let last_size = vec.last().unwrap().size() as u32;
        vec.first_mut().unwrap().frame_size_prev = last_size;
        vec.last_mut().unwrap().frame_size_next = first_size;

        for i in 1..vec.len() {
            let (start, rest) = vec.split_at_mut(i);
            let curr = start.last_mut().unwrap();
            let next = rest.first_mut().unwrap();
            curr.frame_size_next = next.size() as u32;
            next.frame_size_next = curr.size() as u32;
        }
    }
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct ThpComponents<'r>
{
    pub component_count: u32,
    #[auto_struct(init = (16, ()))]
    pub component_types: RoArray<'r, u8>,
    #[auto_struct(init = component_types.iter())]
    pub components: IteratorArray<'r, ThpComponent, RoArrayIter<'r, u8>>,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct ThpComponent
{
    #[auto_struct(args)]
    kind: u8,
    #[auto_struct(init = if kind == 0 { Some(()) } else { None })]
    pub video_info: Option<ThpVideoInfo>,
    #[auto_struct(init = if kind == 1 { Some(()) } else { None })]
    pub audio_info: Option<ThpAudioInfo>,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct ThpVideoInfo
{
    pub width: u32,
    pub height: u32,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct ThpAudioInfo
{
    pub channels_count: u32,
    pub frequency: u32,
    pub samples_count: u32,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct ThpFrameData<'r>
{
    #[auto_struct(args)]
    has_audio: bool,
    pub frame_size_next: u32,
    pub frame_size_prev: u32,

    pub video_size: u32,
    #[auto_struct(init = if has_audio { Some(()) } else { None })]
    pub audio_size: Option<u32>,

    #[auto_struct(init = (video_size as usize, ()))]
    pub video_data: RoArray<'r, u8>,
    #[auto_struct(init = audio_size.map(|s| (s as usize, ())))]
    pub audio_data: Option<RoArray<'r, u8>>,

    #[auto_struct(pad_align = 32)]
    _pad: (),
}
