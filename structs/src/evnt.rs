
// This is intentionally incomplete. Only some parts of the data are needed, and
// we should never need to modify it.

use reader_writer::{CStr, FourCC, RoArray};

auto_struct! {
    #[auto_struct(Readable)]
    #[derive(Debug, Clone)]
    pub struct Evnt<'a>
    {
        //#[expect = 1 || 2]
        version: u32,

        loop_event_count: u32,
        loop_events: RoArray<'a, LoopEvent<'a>> = (loop_event_count as usize, ()),

        user_event_count: u32,
        user_events: RoArray<'a, UserEvent<'a>> = (user_event_count as usize, ()),

        effect_event_count: u32,
        effect_events: RoArray<'a, EffectEvent<'a>> = (effect_event_count as usize, ()),
    }
}



auto_struct! {
    #[auto_struct(Readable)]
    #[derive(Debug, Clone)]
    pub struct EventBase<'a>
    {
        unknown0: u16,
        name: CStr<'a>,
        event_type: u16,
        timestamp: AnimTime,
        event_index: u32,
        unknown1: u8,
        weight: f32,
        character_index: i32,
        unknown2: u32,
    }
}


auto_struct! {
    #[auto_struct(Readable)]
    #[derive(Debug, Clone)]
    pub struct AnimTime
    {
        timestamp: f32,
        // XXX Width?
        type_: u32,
    }
}

auto_struct! {
    #[auto_struct(Readable)]
    #[derive(Debug, Clone)]
    pub struct LoopEvent<'a>
    {
        base: EventBase<'a>,
        unknown: u8,
    }
}

auto_struct! {
    #[auto_struct(Readable)]
    #[derive(Debug, Clone)]
    pub struct UserEvent<'a>
    {
        base: EventBase<'a>,
        event_type: u32,
        bone_name: CStr<'a>,
    }
}

auto_struct! {
    #[auto_struct(Readable)]
    #[derive(Debug, Clone)]
    pub struct EffectEvent<'a>
    {
        base: EventBase<'a>,
        frame_count: u32,

        effect_type: FourCC,
        effect_file_id: u32,
        bone_name: CStr<'a>,
        bone_id: u32,
        scale: u32,
        transform_type: u32,
    }
}
