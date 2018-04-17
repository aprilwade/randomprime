
use reader_writer::{Dap, FourCC, LCow, RoArray, LazyArray, Readable, Reader, Writable};

use std::io;
use std::borrow::Cow;

use scly_props;


auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Debug, Clone)]
    pub struct Scly<'a>
    {
        #[expect = FourCC::from_bytes(b"SCLY")]
        magic: FourCC,

        unknown: u32,

        #[derivable = layers.len() as u32]
        layer_count: u32,

        #[derivable: Dap<_, _> = layers.iter().map(&|i: LCow<SclyLayer>| i.size() as u32).into()]
        _layer_sizes: RoArray<'a, u32> = (layer_count as usize, ()),

        layers: LazyArray<'a, SclyLayer<'a>> = (layer_count as usize, ()),
    }
}

auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Debug, Clone)]
    pub struct SclyLayer<'a>
    {
        unknown: u8,

        #[derivable = objects.len() as u32]
        object_count: u32,
        // TODO: Consider using DiffList here. Maybe requires profiling to decide...
        objects: LazyArray<'a, SclyObject<'a>> = (object_count as usize, ()),

        alignment_padding!(32),
    }
}

impl<'a> SclyLayer<'a>
{
    pub fn new() -> SclyLayer<'a>
    {
        SclyLayer {
            unknown: 0,
            objects: vec![].into(),
        }
    }
}

auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Debug, Clone)]
    pub struct SclyObject<'a>
    {
        #[derivable = property_data.object_type()]
        object_type: u8,

        #[derivable = (8 + connections.size() + property_data.size()) as u32]
        instance_size: u32,

        instance_id: u32,

        #[derivable = connections.len() as u32]
        connection_count: u32,
        connections: LazyArray<'a, Connection> = (connection_count as usize, ()),

        property_data: SclyProperty<'a> = (object_type,
                                           (instance_size - 8) as usize - connections.size()),
    }
}

macro_rules! build_scly_property {
    ($($name:ident, $is_check:ident, $accessor:ident, $accessor_mut:ident,)*) => {

        #[derive(Clone, Debug)]
        pub enum SclyProperty<'a>
        {
            Unknown {
                object_type: u8,
                data: Reader<'a>
            },

            $($name(scly_props::$name<'a>),)*
        }

        impl<'a> SclyProperty<'a>
        {
            pub fn object_type(&self) -> u8
            {
                match *self {
                    SclyProperty::Unknown { object_type, .. } => object_type,
                    $(SclyProperty::$name(_) =>
                      <scly_props::$name as SclyPropertyData>::object_type(),)*
                }
            }

            pub fn guess_kind(&mut self)
            {
                let (mut reader, object_type) = match *self {
                    SclyProperty::Unknown { ref data, object_type }
                        => (data.clone(), object_type),
                    _ => return,
                };
                *self = if false {
                    return
                } $(else if object_type == <scly_props::$name as SclyPropertyData>::object_type() {
                    SclyProperty::$name(reader.read(()))
                })* else {
                    return
                };
            }

            $(
                pub fn $is_check(&self) -> bool
                {
                    match *self {
                        SclyProperty::$name(_) => true,
                        SclyProperty::Unknown { object_type, .. } =>
                            object_type == <scly_props::$name as SclyPropertyData>::object_type(),
                        _ => false,
                    }
                }

                pub fn $accessor(&self) -> Option<Cow<scly_props::$name<'a>>>
                {
                    match *self {
                        SclyProperty::$name(ref inst) => Some(Cow::Borrowed(inst)),
                        SclyProperty::Unknown { ref data, object_type, .. } => {
                            if object_type == <scly_props::$name as SclyPropertyData>::object_type() {
                                Some(Cow::Owned(data.clone().read(())))
                            } else {
                                None
                            }
                        },
                        _ => None,
                    }
                }

                pub fn $accessor_mut(&mut self) -> Option<&mut scly_props::$name<'a>>
                {
                    let (mut data, object_type) = match *self {
                        SclyProperty::Unknown { ref data, object_type, .. } =>
                            (data.clone(), object_type),
                        SclyProperty::$name(ref mut inst) => return Some(inst),
                        _ => return None,
                    };
                    if object_type != <scly_props::$name as SclyPropertyData>::object_type() {
                        return None
                    }
                    *self = SclyProperty::$name(data.read(()));
                    match *self {
                        SclyProperty::$name(ref mut inst) => return Some(inst),
                        _ => panic!(),
                    }
                }
            )*
        }

        impl<'a> Readable<'a> for SclyProperty<'a>
        {
            type Args = (u8, usize);
            fn read(reader: Reader<'a>, (otype, size): Self::Args) -> (Self, Reader<'a>)
            {
                let prop = SclyProperty::Unknown {
                    object_type: otype,
                    data: reader.truncated(size),
                };
                (prop, reader.offset(size))
            }

            fn size(&self) -> usize
            {
                match *self {
                    SclyProperty::Unknown { ref data, .. } => data.len(),
                    $(SclyProperty::$name(ref i) => i.size(),)*
                }
            }
        }

        impl<'a> Writable for SclyProperty<'a>
        {
            fn write<W: io::Write>(&self, writer: &mut W) -> io::Result<()>
            {
                match *self {
                    SclyProperty::Unknown { ref data, .. } => writer.write_all(&data),
                    $(SclyProperty::$name(ref i) => i.write(writer),)*
                }
            }
        }

    };
}

build_scly_property!(
    Actor,           is_actor,            as_actor,            as_actor_mut,
    Trigger,         is_trigger,          as_trigger,          as_trigger_mut,
    Timer,           is_timer,            as_timer,            as_timer_mut,
    Sound,           is_sound,            as_sound,            as_sound_mut,
    Dock,            is_dock,             as_dock,             as_dock_mut,
    SpawnPoint,      is_spawn_point,      as_spawn_point,      as_spawn_point_mut,
    MemoryRelay,     is_memory_relay,     as_memory_relay,     as_memory_relay_mut,
    Pickup,          is_pickup,           as_pickup,           as_pickup_mut,
    Platform,        is_platorm,          as_platorm,          as_platorm_mut,
    PointOfInterest, is_point_of_interest,as_point_of_interest,as_point_of_interest_mut,
    Relay,           is_relay,            as_relay,            as_relay_mut,
    HudMemo,         is_hud_memo,         as_hud_memo,         as_hud_memo_mut,
    SpecialFunction, is_special_function, as_special_function, as_special_function_mut,
    PlayerHint,      is_player_hint,      as_player_hint,      as_player_hint_mut,
    PlayerActor,     is_player_actor,     as_player_actor,     as_player_actor_mut,
    StreamedAudio,   is_streamed_audio,   as_streamed_audio,   as_streamed_audio_mut,
    WorldTransporter,is_world_transporter,as_world_transporter,as_world_transporter_mut,
);

pub trait SclyPropertyData
{
    fn object_type() -> u8;
}


auto_struct! {
    #[auto_struct(Readable, FixedSize, Writable)]
    #[derive(Debug, Clone)]
    pub struct Connection
    {
        state: u32,
        message: u32,
        target_object_id: u32,
    }
}
