use auto_struct_macros::auto_struct;

use reader_writer::{CStr, FourCC, LazyArray, RoArray};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Evnt<'r>
{
    #[auto_struct(derive = if sound_events.is_none() { 1 } else { 2 })]
    version: u32,

    #[auto_struct(derive = loop_events.len() as u32)]
    pub loop_event_count: u32,
    #[auto_struct(init = (loop_event_count as usize, ()))]
    pub loop_events: RoArray<'r, LoopEvent<'r>>,

    #[auto_struct(derive = user_events.len() as u32)]
    pub user_event_count: u32,
    #[auto_struct(init = (user_event_count as usize, ()))]
    pub user_events: RoArray<'r, UserEvent<'r>>,

    #[auto_struct(derive = effect_events.len() as u32)]
    pub effect_event_count: u32,
    #[auto_struct(init = (effect_event_count as usize, ()))]
    pub effect_events: LazyArray<'r, EffectEvent<'r>>,

    #[auto_struct(init = if version == 1 { None } else { Some(()) })]
    #[auto_struct(derive = sound_events.as_ref().map(|a| a.len() as u32))]
    pub sound_event_count: Option<u32>,
    #[auto_struct(init = sound_event_count.map(|i| (i as usize, ())))]
    pub sound_events: Option<RoArray<'r, SoundEvent<'r>>>,

    #[auto_struct(pad_align = 32)]
    _pad: (),
}



#[auto_struct(Readable, Writable)]
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


#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct AnimTime
{
    pub timestamp: f32,
    pub differential_state: u32,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct LoopEvent<'r>
{
    pub base: EventBase<'r>,
    pub unknown: u8,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct UserEvent<'r>
{
    pub base: EventBase<'r>,
    pub event_type: u32,
    pub bone_name: CStr<'r>,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct EffectEvent<'r>
{
    pub base: EventBase<'r>,
    pub frame_count: u32,

    pub effect_type: FourCC,
    pub effect_file_id: u32,
    pub bone_name: CStr<'r>,
    pub scale: u32,
    pub transform_type: u32,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct SoundEvent<'r>
{
    pub base: EventBase<'r>,
    pub sound_id: u32,

    pub reference_amplitude: f32,
    pub reference_distance: f32,
}
