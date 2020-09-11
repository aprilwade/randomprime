use serde::{Deserialize};
use std::cmp;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct StartingItems
{
    pub scan_visor: bool,
    pub missiles: u8,
    pub energy_tanks: u8,
    pub power_bombs: u8,
    pub wave: bool,
    pub ice: bool,
    pub plasma: bool,
    pub charge: bool,
    pub morph_ball: bool,
    pub bombs: bool,
    pub spider_ball: bool,
    pub boost_ball: bool,
    pub varia_suit: bool,
    pub gravity_suit: bool,
    pub phazon_suit: bool,
    pub thermal_visor: bool,
    pub xray: bool,
    pub space_jump: bool,
    pub grapple: bool,
    pub super_missile: bool,
    pub wavebuster: bool,
    pub ice_spreader: bool,
    pub flamethrower: bool,
}

impl StartingItems
{
    pub fn from_u64(mut starting_items: u64) -> Self
    {
        let mut fetch_bits = move |bits: u8| {
            let ret = starting_items & ((1 << bits) - 1);
            starting_items >>= bits;
            ret as u8
        };

        StartingItems {
            scan_visor:  fetch_bits(1) == 1,
            missiles:  fetch_bits(8),
            energy_tanks:  fetch_bits(4),
            power_bombs:  fetch_bits(4),
            wave:  fetch_bits(1) == 1,
            ice:  fetch_bits(1) == 1,
            plasma:  fetch_bits(1) == 1,
            charge:  fetch_bits(1) == 1,
            morph_ball:  fetch_bits(1) == 1,
            bombs:  fetch_bits(1) == 1,
            spider_ball:  fetch_bits(1) == 1,
            boost_ball:  fetch_bits(1) == 1,
            varia_suit:  fetch_bits(1) == 1,
            gravity_suit:  fetch_bits(1) == 1,
            phazon_suit:  fetch_bits(1) == 1,
            thermal_visor:  fetch_bits(1) == 1,
            xray:  fetch_bits(1) == 1,
            space_jump:  fetch_bits(1) == 1,
            grapple:  fetch_bits(1) == 1,
            super_missile:  fetch_bits(1) == 1,
            wavebuster:  fetch_bits(1) == 1,
            ice_spreader:  fetch_bits(1) == 1,
            flamethrower:  fetch_bits(1) == 1,
        }
    }

    pub fn update_spawn_point(&self, spawn_point: &mut structs::SpawnPoint)
    {
        spawn_point.scan_visor = self.scan_visor as u32;
        spawn_point.missiles = self.missiles as u32;
        spawn_point.energy_tanks = self.energy_tanks as u32;
        spawn_point.power_bombs = self.power_bombs as u32;
        spawn_point.wave = self.wave as u32;
        spawn_point.ice = self.ice as u32;
        spawn_point.plasma = self.plasma as u32;
        spawn_point.charge = self.charge as u32;
        spawn_point.morph_ball = self.morph_ball as u32;
        spawn_point.bombs = self.bombs as u32;
        spawn_point.spider_ball = self.spider_ball as u32;
        spawn_point.boost_ball = self.boost_ball as u32;
        spawn_point.varia_suit = self.varia_suit as u32;
        spawn_point.gravity_suit = self.gravity_suit as u32;
        spawn_point.phazon_suit = self.phazon_suit as u32;
        spawn_point.thermal_visor = self.thermal_visor as u32;
        spawn_point.xray = self.xray as u32;
        spawn_point.space_jump = self.space_jump as u32;
        spawn_point.grapple = self.grapple as u32;
        spawn_point.super_missile = self.super_missile as u32;
        spawn_point.wavebuster = self.wavebuster as u32;
        spawn_point.ice_spreader = self.ice_spreader as u32;
        spawn_point.flamethrower = self.flamethrower as u32;
    }

    /// Custom deserializataion function that accepts an int as well as the usual struct/object
    /// version
    pub fn custom_deserialize<'de, D>(deserializer: D) -> Result<Self, D::Error>
        where D: serde::Deserializer<'de>
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        pub enum Wrapper
        {
            Int(u64),
            Struct(StartingItems),
        }

        match <Wrapper as Deserialize>::deserialize(deserializer) {
            Ok(Wrapper::Struct(s)) => Ok(s),
            Ok(Wrapper::Int(i)) => Ok(StartingItems::from_u64(i)),
            Err(e) => Err(e)
        }
    }
    
    pub fn merge(manual_starting_items: StartingItems, random_starting_items: StartingItems) -> Self
    {
        StartingItems {
            scan_visor: manual_starting_items.scan_visor | random_starting_items.scan_visor,
            missiles: cmp::min(manual_starting_items.missiles + random_starting_items.missiles, 250),
            energy_tanks: cmp::min(manual_starting_items.energy_tanks + random_starting_items.energy_tanks, 14),
            power_bombs: cmp::min(manual_starting_items.power_bombs + random_starting_items.power_bombs, 8),
            wave: manual_starting_items.wave | random_starting_items.wave,
            ice: manual_starting_items.ice | random_starting_items.ice,
            plasma: manual_starting_items.plasma | random_starting_items.plasma,
            charge: manual_starting_items.charge | random_starting_items.charge,
            morph_ball: manual_starting_items.morph_ball | random_starting_items.morph_ball,
            bombs: manual_starting_items.bombs | random_starting_items.bombs,
            spider_ball: manual_starting_items.spider_ball | random_starting_items.spider_ball,
            boost_ball: manual_starting_items.boost_ball | random_starting_items.boost_ball,
            varia_suit: manual_starting_items.varia_suit | random_starting_items.varia_suit,
            gravity_suit: manual_starting_items.gravity_suit | random_starting_items.gravity_suit,
            phazon_suit: manual_starting_items.phazon_suit | random_starting_items.phazon_suit,
            thermal_visor: manual_starting_items.thermal_visor | random_starting_items.thermal_visor,
            xray: manual_starting_items.xray | random_starting_items.xray,
            space_jump: manual_starting_items.space_jump | random_starting_items.space_jump,
            grapple: manual_starting_items.grapple | random_starting_items.grapple,
            super_missile: manual_starting_items.super_missile | random_starting_items.super_missile,
            wavebuster: manual_starting_items.wavebuster | random_starting_items.wavebuster,
            ice_spreader: manual_starting_items.ice_spreader | random_starting_items.ice_spreader,
            flamethrower: manual_starting_items.flamethrower | random_starting_items.flamethrower,
        }
    }
    
    pub fn is_empty(&self) -> bool
    {
        !self.scan_visor &&
        self.missiles == 0 &&
        self.energy_tanks == 0 &&
        self.power_bombs == 0 &&
        !self.wave &&
        !self.ice &&
        !self.plasma &&
        !self.charge &&
        !self.morph_ball &&
        !self.bombs &&
        !self.spider_ball &&
        !self.boost_ball &&
        !self.varia_suit &&
        !self.gravity_suit &&
        !self.phazon_suit &&
        !self.thermal_visor &&
        !self.xray &&
        !self.space_jump &&
        !self.grapple &&
        !self.super_missile &&
        !self.wavebuster &&
        !self.ice_spreader &&
        !self.flamethrower
    }
}

impl Default for StartingItems
{
    fn default() -> Self
    {
        StartingItems {
            scan_visor: true,
            missiles: 0,
            energy_tanks: 0,
            power_bombs: 0,
            wave: false,
            ice: false,
            plasma: false,
            charge: false,
            morph_ball: false,
            bombs: false,
            spider_ball: false,
            boost_ball: false,
            varia_suit: false,
            gravity_suit: false,
            phazon_suit: false,
            thermal_visor: false,
            xray: false,
            space_jump: false,
            grapple: false,
            super_missile: false,
            wavebuster: false,
            ice_spreader: false,
            flamethrower: false,
        }
    }
}
