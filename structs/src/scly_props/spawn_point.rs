use auto_struct_macros::auto_struct;

use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use crate::SclyPropertyData;

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct SpawnPoint<'r>
{
    #[auto_struct(expect = 35)]
    prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,

    pub power: u32,
    pub ice: u32,
    pub wave: u32,
    pub plasma: u32,

    pub missiles: u32,
    pub scan_visor: u32,
    pub bombs: u32,
    pub power_bombs: u32,
    pub flamethrower: u32,
    pub thermal_visor: u32,
    pub charge: u32,
    pub super_missile: u32,
    pub grapple: u32,
    pub xray: u32,
    pub ice_spreader: u32,
    pub space_jump: u32,
    pub morph_ball: u32,
    pub combat_visor: u32,
    pub boost_ball: u32,
    pub spider_ball: u32,
    pub power_suit: u32,
    pub gravity_suit: u32,
    pub varia_suit: u32,
    pub phazon_suit: u32,
    pub energy_tanks: u32,
    pub unknown0: u32,
    pub health_refill: u32,
    pub unknown1: u32,
    pub wavebuster: u32,

    pub default_spawn: u8,
    pub active: u8,
    pub morphed: u8,
}

impl<'r> SclyPropertyData for SpawnPoint<'r>
{
    const OBJECT_TYPE: u8 = 0x0F;
}
