#![allow(unused)]

use serde::Deserialize;
use enum_map::{Enum, EnumMap};
use crate::{pickup_meta::{self, PickupType}};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum World {
    FrigateOrpheon,
    TallonOverworld,
    ChozoRuins,
    MagmoorCaverns,
    PhendranaDrifts,
    PhazonMines,
    ImpactCrater,
}

impl World {
    pub fn iter() -> impl Iterator<Item = World>
    {
        [
            World::FrigateOrpheon,
            World::ChozoRuins,
            World::PhendranaDrifts,
            World::TallonOverworld,
            World::PhazonMines,
            World::MagmoorCaverns,
            World::ImpactCrater,
        ].iter().map(|i| *i)
    }

    pub fn to_pak_str(&self) -> &'static str
    {
        match self {
            World::FrigateOrpheon  => "Metroid1.pak",
            World::ChozoRuins      => "Metroid2.pak",
            World::PhendranaDrifts => "Metroid3.pak",
            World::TallonOverworld => "Metroid4.pak",
            World::PhazonMines     => "metroid5.pak",
            World::MagmoorCaverns  => "Metroid6.pak",
            World::ImpactCrater    => "Metroid7.pak",
        }
    }

    pub fn from_pak(pak_str: &str) -> Option<Self> {
        for world in World::iter() {
            if pak_str == world.to_pak_str() {
                return Some(world);
            }
        }

        None
    }

    pub fn mlvl(&self) -> u32 {
        match self {
            World::FrigateOrpheon  => 0x158efe17,
            World::ChozoRuins      => 0x83f6ff6f,
            World::PhendranaDrifts => 0xa8be6291,
            World::TallonOverworld => 0x39f2de28,
            World::PhazonMines     => 0xb1ac4d65,
            World::MagmoorCaverns  => 0x3ef8237c,
            World::ImpactCrater    => 0xc13b09d1,
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            World::FrigateOrpheon  => "Frigate Orpheon",
            World::ChozoRuins      => "Chozo Ruins",
            World::PhendranaDrifts => "Phendrana Drifts",
            World::TallonOverworld => "Tallon Overworld",
            World::PhazonMines     => "Mines, Phazon",
            World::MagmoorCaverns  => "Magmoor Caverns",
            World::ImpactCrater    => "Crater, Impact",
        }
    }

    pub fn to_json_key(&self) -> &'static str {
        match self {
            World::FrigateOrpheon  => "Frigate Orpheon",
            World::ChozoRuins      => "Chozo Ruins",
            World::PhendranaDrifts => "Phendrana Drifts",
            World::TallonOverworld => "Tallon Overworld",
            World::PhazonMines     => "Phazon Mines",
            World::MagmoorCaverns  => "Magmoor Caverns",
            World::ImpactCrater    => "Impact Crater",
        }
    }

    pub fn from_json_key(string: &str) -> Self {
        for world in World::iter() {
            if string.trim().to_lowercase() == world.to_json_key().to_lowercase() {
                return world;
            }
        }

        panic!("Unknown World - '{}'", string);
    }
}

macro_rules! decl_elevators {
    ($($name:ident => { $($contents:tt)* },)*) => {

        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Enum)]
        #[serde(rename_all = "camelCase")]
        pub enum Elevator
        {
            $($name,)*
        }

        impl Elevator
        {
            pub fn elevator_data(&self) -> &'static ElevatorData
            {
                match self {
                    $(Elevator::$name => &ElevatorData { $($contents)* },)*
                }
            }

            fn spawn_room_data(&self) -> &'static SpawnRoomData
            {
                match self {
                    $(Elevator::$name => {
                        const ELV_DATA: ElevatorData = ElevatorData { $($contents)* };
                        &SpawnRoomData {
                            pak_name: ELV_DATA.pak_name,
                            mlvl: ELV_DATA.mlvl,
                            mrea: ELV_DATA.mrea,
                            mrea_idx: ELV_DATA.mrea_idx,
                            room_id: ELV_DATA.room_id,

                            name: ELV_DATA.name,
                        }
                    },)*
                }
            }

            pub fn from_u32(i: u32) -> Option<Self>
            {
                #![allow(non_upper_case_globals)]
                // XXX Counting idents in a macro is a hard problem, so this is a silly workaround
                enum Consts { $($name,)* }
                $(const $name: u32 = Consts::$name as u32;)*
                match i {
                    $($name => Some(Elevator::$name),)*
                    _ => None,
                }
            }

            pub fn iter() -> impl Iterator<Item = Self>
            {
                const ELEVATORS: &[Elevator] = &[
                    $(Elevator::$name,)*
                ];
                ELEVATORS.iter().copied()
            }

            const NUMBERED_ELEVATOR_COUNT: u32 = {
                enum Consts {
                    $($name,)*
                    Max
                }
                Consts::Max as u32
            };
        }
    };
}

impl Elevator
{
    pub fn from_str(name: &str) -> Option<Self> {
        let mut name = name.to_lowercase().replace("\0","");
        name.retain(|c| !c.is_whitespace());
        for elevator in Elevator::iter() {
            let mut elevator_name = elevator.name.to_lowercase().replace("\0","");
            elevator_name.retain(|c| !c.is_whitespace());
            if elevator_name == name {
                return Some(elevator);
            }
        }

        None
    }
}

impl std::ops::Deref for Elevator
{
    type Target = ElevatorData;
    fn deref(&self) -> &Self::Target
    {
        self.elevator_data()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ElevatorData {
    pub pak_name: &'static str,
    pub name: &'static str,
    pub mlvl: u32,
    pub mrea: u32,
    pub mrea_idx: u32,
    pub scly_id: u32,
    pub room_id: u32,

    pub room_strg: u32,
    pub hologram_strg: u32,
    pub control_strg: u32,

    pub default_dest: Elevator,
}

decl_elevators! {
    ChozoRuinsWestMainPlaza => {
        pak_name: "Metroid2.pak",
        name: "Chozo Ruins West\0(Main Plaza)",// "Transport to Tallon Overworld North",
        mlvl: 0x83f6ff6f,
        mrea: 0x3e6b2bb7,
        mrea_idx: 0,
        scly_id: 0x007d,
        room_id: 0xDBED08BA,

        room_strg: 0xF747143D,
        hologram_strg: 0xD3F29D19,
        control_strg: 0x3C6FF426,

        default_dest: Elevator::TallonOverworldNorthTallonCanyon,
    },
    ChozoRuinsNorthSunTower => {
        pak_name: "Metroid2.pak",
        name: "Chozo Ruins North\0(Sun Tower)",// "Transport to Magmoor Caverns North",
        mlvl: 0x83f6ff6f,
        mrea: 0x8316edf5,
        mrea_idx: 24,
        scly_id: 0x180027,
        room_id: 0x372F1027,

        room_strg: 0x71D36693,
        hologram_strg: 0xB4B44968,
        control_strg: 0xC610DFE6,

        default_dest: Elevator::MagmoorCavernsNorthLavaLake,
    },
    ChozoRuinsEastReflectingPoolSaveStation => {
        pak_name: "Metroid2.pak",
        name: "Chozo Ruins East\0(Reflecting Pool, Save Station)",// "Transport to Tallon Overworld East",
        mlvl: 0x83f6ff6f,
        mrea: 0xa5fa69a1,
        mrea_idx: 62,
        scly_id: 0x3e002c,
        room_id: 0xC705A398,

        room_strg: 0x1CE1DDBC,
        hologram_strg: 0x598EF87A,
        control_strg: 0xFCD69EB0,

        default_dest: Elevator::TallonOverworldEastFrigateCrashSite,
    },
    ChozoRuinsSouthReflectingPoolFarEnd => {
        pak_name: "Metroid2.pak",
        name: "Chozo Ruins South\0(Reflecting Pool, Far End)",// "Transport to Tallon Overworld South",
        mlvl: 0x83f6ff6f,
        mrea: 0x236e1b0f,
        mrea_idx: 63,
        scly_id: 0x3f0028,
        room_id: 0x23F35FE1,

        room_strg: 0x9A75AF12,
        hologram_strg: 0x48F39203,
        control_strg: 0x411CF27E,

        default_dest: Elevator::TallonOverworldSouthGreatTreeHallUpper,
    },

    PhendranaDriftsNorthPhendranaShorelines => {
        pak_name: "Metroid3.pak",
        name: "Phendrana Drifts North\0(Phendrana Shorelines)",// "Transport to Magmoor Caverns West",
        mlvl: 0xa8be6291,
        mrea: 0xc00e3781,
        mrea_idx: 0,
        scly_id: 0x002d,
        room_id: 0xB2E861AC,

        room_strg: 0xF7D14F4D,
        hologram_strg: 0x38F9BAC5,
        control_strg: 0x2DDB22E1,

        default_dest: Elevator::MagmoorCavernsWestMonitorStation,
    },
    PhendranaDriftsSouthQuarantineCave => {
        pak_name: "Metroid3.pak",
        name: "Phendrana Drifts South\0(Quarantine Cave)",// "Transport to Magmoor Caverns South",
        mlvl: 0xa8be6291,
        mrea: 0xdd0b0739,
        mrea_idx: 29,
        scly_id: 0x1d005a,
        room_id: 0x31D08ACB,

        room_strg: 0xEAD47FF5,
        hologram_strg: 0x0CEE0B66,
        control_strg: 0x993CEFE8,

        default_dest: Elevator::MagmoorCavernsSouthMagmoorWorkstationSaveStation,
    },

    TallonOverworldNorthTallonCanyon => {
        pak_name: "Metroid4.pak",
        name: "Tallon Overworld North\0(Tallon Canyon)",// "Transport to Chozo Ruins West",
        mlvl: 0x39f2de28,
        mrea: 0x11a02448,
        mrea_idx: 14,
        scly_id: 0xe0005,
        room_id: 0x6FD3B9AB,

        room_strg: 0x9EE2172A,
        hologram_strg: 0x04685AE9,
        control_strg: 0x73A833EB,

        default_dest: Elevator::ChozoRuinsWestMainPlaza,
    },
    ArtifactTemple => {
        pak_name: "Metroid4.pak",
        name: "Artifact Temple",
        mlvl: 0x39f2de28,
        mrea: 0x2398e906,
        mrea_idx: 16,
        scly_id: 0x1002da,
        room_id: 0xCD2B0EA2,

        room_strg: 0xFFFFFFFF,
        hologram_strg: 0xFFFFFFFF,
        control_strg: 0xFFFFFFFF,

        default_dest: Elevator::CraterEntryPoint,
    },
    TallonOverworldEastFrigateCrashSite => {
        pak_name: "Metroid4.pak",
        name: "Tallon Overworld East\0(Frigate Crash Site)",// "Transport to Chozo Ruins East",
        mlvl: 0x39f2de28,
        mrea: 0x8a31665e,
        mrea_idx: 22,
        scly_id: 0x160038,
        room_id: 0xB0C789B5,

        room_strg: 0x0573553C,
        hologram_strg: 0x55A27CA9,
        control_strg: 0x51DCA8D9,

        default_dest: Elevator::ChozoRuinsEastReflectingPoolSaveStation,
    },
    TallonOverworldWestRootCave => {
        pak_name: "Metroid4.pak",
        name: "Tallon Overworld West\0(Root Cave)",// "Transport to Magmoor Caverns East",
        mlvl: 0x39f2de28,
        mrea: 0x15d6ff8b,
        mrea_idx: 23,
        scly_id: 0x170032,
        room_id: 0x6D105C48,

        room_strg: 0xF92C2264,
        hologram_strg: 0xD658ADBD,
        control_strg: 0x8EA61E34,

        default_dest: Elevator::MagmoorCavernsEastTwinFires,
    },
    TallonOverworldSouthGreatTreeHallUpper => {
        pak_name: "Metroid4.pak",
        name: "Tallon Overworld South\0(Great Tree Hall, Upper)",// "Transport to Chozo Ruins South",
        mlvl: 0x39f2de28,
        mrea: 0xca514f0,
        mrea_idx: 41,
        scly_id: 0x290024,
        room_id: 0x5301E9D,

        room_strg: 0x630EA5FC,
        hologram_strg: 0xCC401AA8,
        control_strg: 0xEC16C417,

        default_dest: Elevator::ChozoRuinsSouthReflectingPoolFarEnd,
    },
    TallonOverworldSouthGreatTreeHallLower => {
        pak_name: "Metroid4.pak",
        name: "Tallon Overworld South\0(Great Tree Hall, Lower)",// "Transport to Phazon Mines East",
        mlvl: 0x39f2de28,
        mrea: 0x7d106670,
        mrea_idx: 43,
        scly_id: 0x2b0023,
        room_id: 0xBC2A964C,

        room_strg: 0xF2525512,
        hologram_strg: 0x4921B661,
        control_strg: 0x294EC2B2,

        default_dest: Elevator::PhazonMinesEastMainQuarry,
    },

    PhazonMinesEastMainQuarry => {
        pak_name: "metroid5.pak",
        name: "Phazon Mines East\0(Main Quarry)",// "Transport to Tallon Overworld South",
        mlvl: 0xb1ac4d65,
        mrea: 0x430e999c,
        mrea_idx: 0,
        scly_id: 0x001c,
        room_id: 0x2AC6EC36,

        room_strg: 0x8D7B16B4,
        hologram_strg: 0xB60F6ADF,
        control_strg: 0xA00EF446,

        default_dest: Elevator::TallonOverworldSouthGreatTreeHallLower,
    },
    PhazonMinesWestPhazonProcessingCenter => {
        pak_name: "metroid5.pak",
        name: "Phazon Mines West\0(Phazon Processing Center)",// "Transport to Magmoor Caverns South",
        mlvl: 0xb1ac4d65,
        mrea: 0xe2c2cf38,
        mrea_idx: 25,
        scly_id: 0x190011,
        room_id: 0x91C144BF,

        room_strg: 0x47C4108D,
        hologram_strg: 0xDFD2AE6D,
        control_strg: 0x1D8BB16C,

        default_dest: Elevator::MagmoorCavernsSouthMagmoorWorkstationDebris,
    },

    MagmoorCavernsNorthLavaLake => {
        pak_name: "Metroid6.pak",
        name: "Magmoor Caverns North\0(Lava Lake)",// "Transport to Chozo Ruins North",
        mlvl: 0x3ef8237c,
        mrea: 0x3beaadc9,
        mrea_idx: 0,
        scly_id: 0x001f,
        room_id: 0x7DC0D75B,

        room_strg: 0x1BEFC19B,
        hologram_strg: 0x8EA3FD98,
        control_strg: 0x0D3EC7DC,

        default_dest: Elevator::ChozoRuinsNorthSunTower,
    },
    MagmoorCavernsWestMonitorStation => {
        pak_name: "Metroid6.pak",
        name: "Magmoor Caverns West\0(Monitor Station)",// "Transport to Phendrana Drifts North",
        mlvl: 0x3ef8237c,
        mrea: 0xdca9a28b,
        mrea_idx: 13,
        scly_id: 0xd0022,
        room_id: 0x4318F156,

        room_strg: 0xE0E1C4DA,
        hologram_strg: 0x4F2D2258,
        control_strg: 0xD0A81E59,

        default_dest: Elevator::PhendranaDriftsNorthPhendranaShorelines,
    },
    MagmoorCavernsEastTwinFires => {
        pak_name: "Metroid6.pak",
        name: "Magmoor Caverns East\0(Twin Fires)",// "Transport to Tallon Overworld West",
        mlvl: 0x3ef8237c,
        mrea: 0x4c3d244c,
        mrea_idx: 16,
        scly_id: 0x100020,
        room_id: 0xB3128CF6,

        room_strg: 0xBD4E14B9,
        hologram_strg: 0x58DA42EA,
        control_strg: 0x4BE9A4CC,

        default_dest: Elevator::TallonOverworldWestRootCave,
    },
    MagmoorCavernsSouthMagmoorWorkstationDebris => {
        pak_name: "Metroid6.pak",
        name: "Magmoor Caverns South\0(Magmoor Workstation, Debris)",// "Transport to Phazon Mines West",
        mlvl: 0x3ef8237c,
        mrea: 0xef2f1440,
        mrea_idx: 26,
        scly_id: 0x1a0024,
        room_id: 0x921FFEDB,

        room_strg: 0xFF5F6594,
        hologram_strg: 0x28E3D615,
        control_strg: 0x2FAF7EDA,

        default_dest: Elevator::PhazonMinesWestPhazonProcessingCenter,
    },
    MagmoorCavernsSouthMagmoorWorkstationSaveStation => {
        pak_name: "Metroid6.pak",
        name: "Magmoor Caverns South\0(Magmoor Workstation, Save Station)",// "Transport to Phendrana Drifts South",
        mlvl: 0x3ef8237c,
        mrea: 0xc1ac9233,
        mrea_idx: 27,
        scly_id: 0x1b0028,
        room_id: 0xC0201A31,

        room_strg: 0x66DEBE97,
        hologram_strg: 0x61805AFF,
        control_strg: 0x6F30E3D4,

        default_dest: Elevator::PhendranaDriftsSouthQuarantineCave,
    },

    CraterEntryPoint => {
        pak_name: "Metroid7.pak",
        name: "Crater Entry Point",
        mlvl: 0xc13b09d1,
        mrea: 0x93668996,
        mrea_idx: 0,
        scly_id: 0x0098,
        room_id: 0x2B878F78,

        room_strg: 0xFFFFFFFF,
        hologram_strg: 0xFFFFFFFF,
        control_strg: 0xFFFFFFFF,

        default_dest: Elevator::ArtifactTemple,
    },
}

macro_rules! decl_spawn_rooms {
    (
        $($name:ident => { $($contents:tt)* },)*
        @Unnumbered:
        $($un_name:ident => { $($un_contents:tt)* },)*
    ) => {

        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub enum SpawnRoom
        {
            Elevator(Elevator),
            $($name,)*
            $($un_name,)*
        }

        impl SpawnRoom
        {
            pub fn spawn_room_data(&self) -> &SpawnRoomData
            {
                match self {
                    SpawnRoom::Elevator(elv) => elv.spawn_room_data(),
                    $(SpawnRoom::$name => &SpawnRoomData { $($contents)* },)*
                    $(SpawnRoom::$un_name => &SpawnRoomData { $($un_contents)* },)*
                }
            }

            pub fn from_u32(i: u32) -> Option<Self>
            {
                #![allow(non_upper_case_globals)]
                if let Some(elv) = Elevator::from_u32(i) {
                    Some(elv.into())
                } else {
                    #[repr(u32)]
                    enum Consts {
                        _Start = Elevator::NUMBERED_ELEVATOR_COUNT - 1,
                        $($name,)*
                    }
                    $(
                        const $name: u32 = Consts::$name as u32;
                    )*
                    match i {
                        $($name => Some(SpawnRoom::$name),)*
                        _ => None
                    }
                }
            }

            pub fn to_str(&self) -> &'static str
            {
                for (pak_name, rooms) in pickup_meta::ROOM_INFO.iter() { // for each pak
                    for room_info in rooms.iter() { // for each room in the pak
                        if self.spawn_room_data().mrea == room_info.room_id.to_u32() {
                            return room_info.name;
                        }
                    }
                }

                panic!("Failed to find a mreaId={} in pickup_meta.rs.in",self.spawn_room_data().mrea)
            }
        }
    };
}

impl SpawnRoomData
{
    pub fn from_str(dest_name: &str) -> Self
    {
        let dest_name = dest_name.to_lowercase();

        // Handle special destinations //
        if dest_name == "credits" {
            return *SpawnRoom::EndingCinematic.spawn_room_data();
        }

        if dest_name == "frigate" {
            return *SpawnRoom::FrigateExteriorDockingHangar.spawn_room_data();
        }

        // Handle elevator destinations //
        if let Some(elevator) = Elevator::from_str(&dest_name) {
            return *elevator.spawn_room_data();
        }

        // Handle specific room destinations //
        let vec: Vec<&str> = dest_name.split(":").collect();
        assert!(vec.len() == 2);
        let world_name = vec[0].trim();
        let room_name = vec[1].trim();

        for (pak_name, rooms) in pickup_meta::ROOM_INFO.iter() { // for each pak
            let world = World::from_pak(pak_name).unwrap();

            if !world.to_str().to_lowercase().starts_with(&world_name) {
                continue;
            }

            let mut idx: u32 = 0;
            for room_info in rooms.iter() { // for each room in the pak
                if room_info.name.to_lowercase() == room_name {

                    return SpawnRoomData {
                        pak_name,
                        mlvl: world.mlvl(),
                        mrea: room_info.room_id.to_u32(),
                        mrea_idx: idx,
                        room_id: 0,
                        name: room_info.name,
                    };
                }
                idx = idx + 1;
            }
        }

        panic!("Error - Could not find destination '{}'", dest_name)
    }
}

impl std::ops::Deref for SpawnRoom
{
    type Target = SpawnRoomData;
    fn deref(&self) -> &Self::Target
    {
        self.spawn_room_data()
    }
}

impl PartialEq<Elevator> for SpawnRoom
{
    fn eq(&self, other: &Elevator) -> bool
    {
        self == &SpawnRoom::Elevator(*other)
    }
}

impl From<Elevator> for SpawnRoom
{
    fn from(elv: Elevator) -> Self
    {
        SpawnRoom::Elevator(elv)
    }
}

impl Default for SpawnRoom
{
    fn default() -> Self
    {
        SpawnRoom::FrigateExteriorDockingHangar
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpawnRoomData
{
    pub pak_name: &'static str,
    pub mlvl: u32,
    pub mrea: u32,
    pub mrea_idx: u32,
    pub room_id: u32,

    pub name: &'static str,
}

impl From<ElevatorData> for SpawnRoomData
{
    fn from(elv: ElevatorData) -> Self
    {
        SpawnRoomData {
            pak_name: elv.pak_name,
            mlvl: elv.mlvl,
            mrea: elv.mrea,
            mrea_idx: elv.mrea_idx,
            room_id: elv.room_id,
            name: elv.name,
        }
    }
}


decl_spawn_rooms! {
    LandingSite => {
        pak_name: "Metroid4.pak",
        mlvl: 0x39f2de28,
        mrea: 0xb2701146,
        mrea_idx: 0,
        room_id: 0x8ff17910,

        name: "Landing Site",
    },

    @Unnumbered:
    EndingCinematic => {
        pak_name: "Metroid8.pak",
        mlvl: 0x13d79165,
        mrea: 0xb4b41c48,
        mrea_idx: 0,
        room_id: 0,

        name: "End of Game",
    },
    FrigateExteriorDockingHangar => {
        pak_name: "Metroid1.pak",
        mlvl: 0x158EFE17,
        mrea: 0xD1241219,
        mrea_idx: 0,
        room_id: 0xC34F20FF,

        name: "Frigate\0(Exterior Docking Hangar)",
    },
}

