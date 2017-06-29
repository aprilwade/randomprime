
use reader_writer::{FourCC, IteratorArray, LazyArray, Readable, RoArray, RoArrayIter};

auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Debug, Clone)]
    pub struct Thp<'a>
    {
        #[expect = b"THP\0".into()]
        magic: FourCC,

        #[expect = 0x00010000]
        version: u32,

        max_buffer_size: u32,
        max_audio_samples: u32,

        #[expect = 0x41efc28f]
        fps: u32,
        #[derivable = frames.len() as u32]
        frame_count: u32,
        #[derivable = frames.iter().next().unwrap().size() as u32]
        _first_frame_size: u32,
        #[derivable = frames.size() as u32]
        _data_size: u32,

        #[expect = 0x30]
        _component_data_offset: u32,
        #[expect = 0]
        _offsets_data_offset: u32,
        #[derivable = 0x30 + components.size() as u32]
        _first_frame_offset: u32,
        #[derivable = (0x30 + components.size() + frames.size() -
                       frames.iter().last().unwrap().size()) as u32]
        _last_frame_offset: u32,

        components: ThpComponents<'a>,
        frames: LazyArray<'a, ThpFrameData<'a>> = (frame_count as usize,
                                                   components.component_count > 1),
    }
}

impl<'a> Thp<'a>
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

auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Debug, Clone)]
    pub struct ThpComponents<'a>
    {
        component_count: u32,
        component_types: RoArray<'a, u8> = (16, ()),
        components: IteratorArray<'a, ThpComponent, RoArrayIter<'a, u8>> =
            component_types.iter(),
    }
}

auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Debug, Clone)]
    pub struct ThpComponent
    {
        #[args]
        kind: u8,
        video_info: Option<ThpVideoInfo> = if kind == 0 { Some(()) } else { None },
        audio_info: Option<ThpAudioInfo> = if kind == 1 { Some(()) } else { None },
    }
}

auto_struct! {
    #[auto_struct(Readable, Writable, FixedSize)]
    #[derive(Debug, Clone)]
    pub struct ThpVideoInfo
    {
        width: u32,
        height: u32,
    }
}

auto_struct! {
    #[auto_struct(Readable, Writable, FixedSize)]
    #[derive(Debug, Clone)]
    pub struct ThpAudioInfo
    {
        channels_count: u32,
        frequency: u32,
        samples_count: u32,
    }
}

auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Debug, Clone)]
    pub struct ThpFrameData<'a>
    {
        #[args]
        has_audio: bool,
        frame_size_next: u32,
        frame_size_prev: u32,

        video_size: u32,
        audio_size: Option<u32> = if has_audio { Some(()) } else { None },

        video_data: RoArray<'a, u8> = (video_size as usize, ()),
        audio_data: Option<RoArray<'a, u8>> = audio_size.map(|s| (s as usize, ())),

        alignment_padding!(32),
    }
}
