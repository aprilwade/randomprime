
use SclyPropertyData;
use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;

auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Debug, Clone)]
    pub struct SpawnPoint<'a>
    {
        #[expect = 35]
        prop_count: u32,

        name: CStr<'a>,

        position: GenericArray<f32, U3>,
        rotation: GenericArray<f32, U3>,

        power: u32,
        ice: u32,
        wave: u32,
        plasma: u32,

        missiles: u32,
        scan_visor: u32,
        bombs: u32,
        power_bombs: u32,
        flamethrower: u32,
        thermal_visor: u32,
        charge: u32,
        super_missile: u32,
        grapple: u32,
        xray: u32,
        ice_spreader: u32,
        space_jump: u32,
        morph_ball: u32,
        combat_visor: u32,
        boost_ball: u32,
        spider_ball: u32,
        power_suit: u32,
        gravity_suit: u32,
        varia_suit: u32,
        phazon_suit: u32,
        energy_tanks: u32,
        unknown0: u32,
        health_refill: u32,
        unknown1: u32,
        wavebuster: u32,

        default_spawn: u8,
        active: u8,
        morphed: u8,
    }
}

impl<'a> SclyPropertyData for SpawnPoint<'a>
{
    fn object_type() -> u8
    {
        0x0F
    }
}
