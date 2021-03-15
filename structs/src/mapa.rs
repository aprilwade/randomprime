use auto_struct_macros::auto_struct;

use reader_writer::{LazyArray, RoArray};
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;


#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Mapa<'r>
{
    #[auto_struct(expect = 0xDEADD00D)]
    pub magic: u32,
    #[auto_struct(expect = 2)]
    pub version: u32,

    pub type_: u32,
    pub visibility_mode: u32,
    pub aabb: GenericArray<f32, U6>,

    #[auto_struct(derive = objects.len() as u32)]
    pub object_count: u32,
    #[auto_struct(derive = vertices.len() as u32)]
    pub vertex_count: u32,
    #[auto_struct(derive = surfaces.len() as u32)]
    pub surface_count: u32,

    #[auto_struct(init = (object_count as usize, ()))]
    pub objects: LazyArray<'r, MapaObject>,
    #[auto_struct(init = (vertex_count as usize, ()))]
    pub vertices: LazyArray<'r, GenericArray<f32, U3>>,

    // TODO: This can be iter derived from surfaces...
    #[auto_struct(init = (surface_count as usize, ()))]
    pub surface_headers: RoArray<'r, MapaSurfaceHeader>,
    #[auto_struct(init = (surface_count as usize, ()))]
    pub surfaces: RoArray<'r, MapaSurface<'r>>,

    #[auto_struct(pad_align = 32)]
    _pad: (),
}

#[derive(Debug, Clone)]
pub enum MapaObjectType
{
    DoorNormal         = 0,
    DoorShield         = 1,
    DoorIce            = 2,
    DoorWave           = 3,
    DoorPlasma         = 4,
    DoorBig            = 5,
    DoorBig2           = 6,
    DoorIceCeiling     = 7,
    DoorIceFloor       = 8,
    DoorWaveCeiling    = 9,
    DoorWaveFloor      = 10,
    DoorPlasmaCeiling  = 11,
    DoorPlasmaFloor    = 12,
    DoorIceFloor2      = 13,
    DoorWaveFloor2     = 14,
    DoorPlasmaFloor2   = 15,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct MapaObject
{
    pub type_: u32,
    pub visibility_mode: u32,
    pub editor_id: u32,
    pub seed1: u32,
    pub transform_matrix: GenericArray<f32, U12>,
    pub seek2: GenericArray<u32, U4>,
}

impl MapaObject
{
    pub fn is_door(&self) -> bool
    {
        self.type_ < 16 && self.type_ > 0
    }

    pub fn is_vertical(&self) -> bool
    {
        self.type_ < 16 && self.type_ > 6
    }
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct MapaSurfaceHeader
{
    pub center: GenericArray<f32, U3>,
    pub center_of_mass: GenericArray<f32, U3>,
    pub primitive_table_start: u32,
    pub border_table_start: u32,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct MapaSurface<'r>
{
    #[auto_struct(derive = primitives.len() as u32)]
    pub primitive_count: u32,
    #[auto_struct(init = (primitive_count as usize, ()))]
    pub primitives: RoArray<'r, MapaPrimitive<'r>>,

    #[auto_struct(derive = borders.len() as u32)]
    pub border_count: u32,
    #[auto_struct(init = (border_count as usize, ()))]
    pub borders: RoArray<'r, MapaBorder<'r>>,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct MapaPrimitive<'r>
{
    pub type_: u32,
    #[auto_struct(derive = indices.len() as u32)]
    pub index_count: u32,
    #[auto_struct(init = (index_count as usize, ()))]
    pub indices: RoArray<'r, u8>,

    #[auto_struct(pad_align = 4)]
    pub _pad: (),
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct MapaBorder<'r>
{
    #[auto_struct(derive = indices.len() as u32)]
    pub index_count: u32,
    #[auto_struct(init = (index_count as usize, ()))]
    pub indices: RoArray<'r, u8>,

    #[auto_struct(pad_align = 4)]
    pub _pad: (),
}
