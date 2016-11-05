
use reader_writer::{Array, ArrayBorrowedIterator, Dap, FourCC, ImmCow, IteratorArray, Readable,
                    Reader, Writable, pad_bytes_count, pad_bytes};

use std::io::Write;


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
        _layer_sizes: Array<'a, u32> = (layer_count as usize, ()),

        layers: Array<'a, SclyLayer<'a>> = (layer_count as usize, ()),
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
        objects: Array<'a, SclyObject<'a>> = (object_count as usize, ()),

        #[offset]
        offset: usize,
        #[derivable = pad_bytes(32, offset)]
        _padding: Array<'a, u8> = (pad_bytes_count(32, offset), ()),
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
        connections: Array<'a, Connection> = (connection_count as usize, ()),

        #[derivable = property_data.property_count()]
        property_count: u32,
        property_data: SclyProperty<'a> = (object_type, property_count, 
                                           (instance_size - 12) as usize - connections.size()),
    }
}

#[derive(Clone, Debug)]
pub enum SclyProperty<'a>
{
    Unknown {
        object_type: u8,
        property_count: u32,
        data: Reader<'a>
    },
}

impl<'a> SclyProperty<'a>
{
    fn object_type(&self) -> u8
    {
        match *self {
            SclyProperty::Unknown { object_type, .. } => object_type,
        }
    }

    fn property_count(&self) -> u32
    {
        match *self {
            SclyProperty::Unknown { property_count, .. } => property_count,
        }
    }
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
        }
    }
}

impl<'a> Writable for SclyProperty<'a>
{
    fn write<W: Write>(&self, writer: &mut W)
    {
        match *self {
            SclyProperty::Unknown { ref data, .. } => writer.write_all(&data).unwrap(),
        }
    }
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
