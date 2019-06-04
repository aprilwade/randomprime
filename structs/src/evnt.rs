use auto_struct_macros::auto_struct;

// This is intentionally incomplete. Only some parts of the data are needed, and
// we should never need to modify it.

use reader_writer::{CStr, FourCC, RoArray};

#[auto_struct(Readable)]
#[derive(Debug, Clone)]
pub struct Evnt<'r>
{
    //#[expect = 1 || 2]
    version: u32,

    pub loop_event_count: u32,
    #[auto_struct(init = (loop_event_count as usize, ()))]
    pub loop_events: RoArray<'r, LoopEvent<'r>>,

    pub user_event_count: u32,
    #[auto_struct(init = (user_event_count as usize, ()))]
    pub user_events: RoArray<'r, UserEvent<'r>>,

    pub effect_event_count: u32,
    #[auto_struct(init = (effect_event_count as usize, ()))]
    pub effect_events: RoArray<'r, EffectEvent<'r>>,
}



#[auto_struct(Readable)]
#[derive(Debug, Clone)]
pub struct EventBase<'r>
{
    pub unknown0: u16,
    pub name: CStr<'r>,
    pub event_type: u16,
    pub timestamp: AnimTime,
    pub event_index: u32,
    pub unknown1: u8,
    pub weight: f32,
    pub character_index: i32,
    pub unknown2: u32,
}


#[auto_struct(Readable)]
#[derive(Debug, Clone)]
pub struct AnimTime
{
    pub timestamp: f32,
    // XXX Width?
    pub type_: u32,
}

#[auto_struct(Readable)]
#[derive(Debug, Clone)]
pub struct LoopEvent<'r>
{
    pub base: EventBase<'r>,
    pub unknown: u8,
}

#[auto_struct(Readable)]
#[derive(Debug, Clone)]
pub struct UserEvent<'r>
{
    pub base: EventBase<'r>,
    pub event_type: u32,
    pub bone_name: CStr<'r>,
}

#[auto_struct(Readable)]
#[derive(Debug, Clone)]
pub struct EffectEvent<'r>
{
    pub base: EventBase<'r>,
    pub frame_count: u32,

    pub effect_type: FourCC,
    pub effect_file_id: u32,
    pub bone_name: CStr<'r>,
    pub bone_id: u32,
    pub scale: u32,
    pub transform_type: u32,
}
