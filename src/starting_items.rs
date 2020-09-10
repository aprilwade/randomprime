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
    
    pub fn merge(&self, starting_items_: &StartingItems) -> StartingItems
    {
        StartingItems {
            scan_visor: self.scan_visor | starting_items_.scan_visor,
            missiles: cmp::min(self.missiles + starting_items_.missiles, 250),
            energy_tanks: cmp::min(self.energy_tanks + starting_items_.energy_tanks, 14),
            power_bombs: cmp::min(self.power_bombs + starting_items_.power_bombs, 8),
            wave: self.wave | starting_items_.wave,
            ice: self.ice | starting_items_.ice,
            plasma: self.plasma | starting_items_.plasma,
            charge: self.charge | starting_items_.charge,
            morph_ball: self.morph_ball | starting_items_.morph_ball,
            bombs: self.bombs | starting_items_.bombs,
            spider_ball: self.spider_ball | starting_items_.spider_ball,
            boost_ball: self.boost_ball | starting_items_.boost_ball,
            varia_suit: self.varia_suit | starting_items_.varia_suit,
            gravity_suit: self.gravity_suit | starting_items_.gravity_suit,
            phazon_suit: self.phazon_suit | starting_items_.phazon_suit,
            thermal_visor: self.thermal_visor | starting_items_.thermal_visor,
            xray: self.xray | starting_items_.xray,
            space_jump: self.space_jump | starting_items_.space_jump,
            grapple: self.grapple | starting_items_.grapple,
            super_missile: self.super_missile | starting_items_.super_missile,
            wavebuster: self.wavebuster | starting_items_.wavebuster,
            ice_spreader: self.ice_spreader | starting_items_.ice_spreader,
            flamethrower: self.flamethrower | starting_items_.flamethrower,
        }
    }
    
    pub fn empty() -> Self
    {
        let mut _starting_items = StartingItems::default();
        _starting_items.scan_visor = false;
        
        _starting_items
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
