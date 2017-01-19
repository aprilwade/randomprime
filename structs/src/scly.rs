
use reader_writer::{Dap, FourCC, ImmCow, RoArray, LazyArray, Readable, Reader, Writable,
                    pad_bytes_count, pad_bytes};

use std::io::Write;
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

        #[derivable: Dap<_, _> = layers.iter().map(&|i: ImmCow<SclyLayer>| i.size() as u32).into()]
        _layer_sizes: RoArray<'a, u32> = (layer_count as usize, ()),

        layers: LazyArray<'a, SclyLayer<'a>> = (layer_count as usize, ()),
        // TODO: If we wrap SclyLayer in LazySized, then we can make use of the
        //       layer_sizes field to maybe speed things up. It probably requires
        //       profiling to see if its actually any better.
        //layers: IteratorArray<'a, SclyLayer<'a>, ArrayBorrowedIterator<'a, u32>> = layer_sizes.borrowed_iter().unwrap(),
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

        #[offset]
        offset: usize,
        #[derivable = pad_bytes(32, offset)]
        _padding: RoArray<'a, u8> = (pad_bytes_count(32, offset), ()),
    }
}

impl<'a> SclyLayer<'a>
{
    pub fn new() -> SclyLayer<'a>
    {
        SclyLayer {
            unknown: 0,
            objects: LazyArray::Owned(Vec::new()),
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

        #[derivable = (12 + connections.size() + property_data.size()) as u32]
        instance_size: u32,

        instance_id: u32,

        #[derivable = connections.len() as u32]
        connection_count: u32,
        connections: LazyArray<'a, Connection> = (connection_count as usize, ()),

        #[derivable = property_data.property_count()]
        property_count: u32,
        property_data: SclyProperty<'a> = (object_type, property_count,
                                           (instance_size - 12) as usize - connections.size()),
    }
}

macro_rules! build_scly_property {
    ($($name:ident, $accessor:ident, $accessor_mut:ident, $obj_type:expr, $prop_count:expr,)*) => {

        #[derive(Clone, Debug)]
        pub enum SclyProperty<'a>
        {
            Unknown {
                object_type: u8,
                property_count: u32,
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
                    $(SclyProperty::$name(_) => $obj_type,)*
                }
            }

            fn property_count(&self) -> u32
            {
                match *self {
                    SclyProperty::Unknown { property_count, .. } => property_count,
                    $(SclyProperty::$name(_) => $prop_count,)*
                }
            }

            pub fn guess_kind(&mut self)
            {
                let (mut reader, object_type, prop_count) = match *self {
                    SclyProperty::Unknown { ref data, object_type, property_count }
                        => (data.clone(), object_type, property_count),
                    _ => return,
                };
                *self = match object_type {
                    $($obj_type => {
                        assert_eq!(prop_count, $prop_count);
                        SclyProperty::$name(reader.read(()))
                    },)*
                    _ => return,
                }
            }

            $(
                pub fn $accessor(&self) -> Option<Cow<scly_props::$name<'a>>>
                {
                    match *self {
                        SclyProperty::$name(ref inst) => Some(Cow::Borrowed(inst)),
                        SclyProperty::Unknown { ref data, object_type, .. } => {
                            if object_type == $obj_type {
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
                    self.guess_kind();
                    match *self {
                        SclyProperty::$name(ref mut inst) => Some(inst),
                        _ => None,
                    }
                }
            )*
        }

        impl<'a> Readable<'a> for SclyProperty<'a>
        {
            type Args = (u8, u32, usize);
            fn read(reader: Reader<'a>, (otype, prop_count, size): Self::Args) -> (Self, Reader<'a>)
            {
                let prop = SclyProperty::Unknown {
                    object_type: otype,
                    property_count: prop_count,
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
            fn write<W: Write>(&self, writer: &mut W)
            {
                match *self {
                    SclyProperty::Unknown { ref data, .. } => writer.write_all(&data).unwrap(),
                    $(SclyProperty::$name(ref i) => i.write(writer),)*
                }
            }
        }

    };
}

build_scly_property!(
    Timer,           as_timer,            as_timer_mut,              0x05, 6,
    Sound,           as_sound,            as_sound_mut,              0x09, 20,
    SpawnPoint,      as_spawn_point,      as_spawn_point_mut,        0x0F, 35,
    Pickup,          as_pickup,           as_pickup_mut,             0x11, 18,
    Relay,           as_relay,            as_relay_mut,              0x15, 2,
    HudMemo,         as_hud_memo,         as_hud_memo_mut,           0x17, 6,
    SpecialFunction, as_special_function, as_special_function_mut,   0x3A, 15,
    PlayerHint,      as_player_hint,      as_player_hint_mut,        0x3E, 6,
    StreamedAudio,   as_streamed_audio,   as_streamed_audio_mut,     0x61, 9,
);


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
