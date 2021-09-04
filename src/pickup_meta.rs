use std::mem;

use serde::Deserialize;

use reader_writer::{FourCC, Reader};
use structs::{Connection, ConnectionMsg, ConnectionState, Pickup, ResId, res_id};

use crate::custom_assets::custom_asset_ids;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PickupType
{
    Missile,
    EnergyTank,
    ThermalVisor,
    XRayVisor,
    VariaSuit,
    GravitySuit,
    PhazonSuit,
    MorphBall,
    BoostBall,
    SpiderBall,
    MorphBallBomb,
    PowerBombExpansion,
    PowerBomb,
    ChargeBeam,
    SpaceJumpBoots,
    GrappleBeam,
    SuperMissile,
    Wavebuster,
    IceSpreader,
    Flamethrower,
    WaveBeam,
    IceBeam,
    PlasmaBeam,
    ArtifactOfLifegiver,
    ArtifactOfWild,
    ArtifactOfWorld,
    ArtifactOfSun,
    ArtifactOfElder,
    ArtifactOfSpirit,
    ArtifactOfTruth,
    ArtifactOfChozo,
    ArtifactOfWarrior,
    ArtifactOfNewborn,
    ArtifactOfNature,
    ArtifactOfStrength,
    Nothing,
    ScanVisor,
    PowerBeam,
    UnknownItem1,
    UnknownItem2,
    HealthRefill,
    MissileRefill,
    PowerBombRefill,
    #[serde(skip)]
    ShinyMissile,
}

impl PickupType
{
    pub fn name(&self) -> &'static str
    {
        match self {
            PickupType::Missile =>             "Missile",
            PickupType::EnergyTank =>          "Energy Tank",
            PickupType::ThermalVisor =>        "Thermal Visor",
            PickupType::XRayVisor =>           "X-Ray Visor",
            PickupType::VariaSuit =>           "Varia Suit",
            PickupType::GravitySuit =>         "Gravity Suit",
            PickupType::PhazonSuit =>          "Phazon Suit",
            PickupType::MorphBall =>           "Morph Ball",
            PickupType::BoostBall =>           "Boost Ball",
            PickupType::SpiderBall =>          "Spider Ball",
            PickupType::MorphBallBomb =>       "Morph Ball Bomb",
            PickupType::PowerBombExpansion =>  "Power Bomb Expansion",
            PickupType::PowerBomb =>           "Power Bomb",
            PickupType::ChargeBeam =>          "Charge Beam",
            PickupType::SpaceJumpBoots =>      "Space Jump Boots",
            PickupType::GrappleBeam =>         "Grapple Beam",
            PickupType::SuperMissile =>        "Super Missile",
            PickupType::Wavebuster =>          "Wavebuster",
            PickupType::IceSpreader =>         "Ice Spreader",
            PickupType::Flamethrower =>        "Flamethrower",
            PickupType::WaveBeam =>            "Wave Beam",
            PickupType::IceBeam =>             "Ice Beam",
            PickupType::PlasmaBeam =>          "Plasma Beam",
            PickupType::ArtifactOfLifegiver => "Artifact of Lifegiver",
            PickupType::ArtifactOfWild =>      "Artifact of Wild",
            PickupType::ArtifactOfWorld =>     "Artifact of World",
            PickupType::ArtifactOfSun =>       "Artifact of Sun",
            PickupType::ArtifactOfElder =>     "Artifact of Elder",
            PickupType::ArtifactOfSpirit =>    "Artifact of Spirit",
            PickupType::ArtifactOfTruth =>     "Artifact of Truth",
            PickupType::ArtifactOfChozo =>     "Artifact of Chozo",
            PickupType::ArtifactOfWarrior =>   "Artifact of Warrior",
            PickupType::ArtifactOfNewborn =>   "Artifact of Newborn",
            PickupType::ArtifactOfNature =>    "Artifact of Nature",
            PickupType::ArtifactOfStrength =>  "Artifact of Strength",
            PickupType::Nothing =>             "Nothing",
            PickupType::ScanVisor =>           "Scan Visor",
            PickupType::PowerBeam =>           "Power Beam",
            PickupType::UnknownItem1 =>        "Unknown Item 1",
            PickupType::UnknownItem2 =>        "Unknown Item 2",
            PickupType::HealthRefill =>        "Health Refill",
            PickupType::MissileRefill =>       "Missile Refill",
            PickupType::PowerBombRefill =>     "Power Bomb Refill",
            PickupType::ShinyMissile =>        "Shiny Missile",
        }
    }

    pub fn is_artifact(&self) -> bool
    {
        match self {
            PickupType::ArtifactOfLifegiver => true,
            PickupType::ArtifactOfWild =>      true,
            PickupType::ArtifactOfWorld =>     true,
            PickupType::ArtifactOfSun =>       true,
            PickupType::ArtifactOfElder =>     true,
            PickupType::ArtifactOfSpirit =>    true,
            PickupType::ArtifactOfTruth =>     true,
            PickupType::ArtifactOfChozo =>     true,
            PickupType::ArtifactOfWarrior =>   true,
            PickupType::ArtifactOfNewborn =>   true,
            PickupType::ArtifactOfNature =>    true,
            PickupType::ArtifactOfStrength =>  true,
            _ => false,
        }
    }

    pub fn skip_hudmemos_strg(&self) -> ResId<res_id::STRG>
    {
        let start = custom_asset_ids::SKIP_HUDMEMO_STRG_START.to_u32();
        let end = custom_asset_ids::SKIP_HUDMEMO_STRG_END.to_u32();
        ResId::new((start..end).nth(self.kind().unwrap_or(0) as usize).unwrap_or(0xFFFFFFFF))
    }

    pub fn pickup_data<'a>(&self) -> Pickup
    {
        Reader::new(self.raw_pickup_data()).read(())
    }

    pub fn kind(&self) -> Option<u32>
    {
        match self {
            PickupType::PowerBeam =>           Some(0),
            PickupType::IceBeam =>             Some(1),
            PickupType::WaveBeam =>            Some(2),
            PickupType::PlasmaBeam =>          Some(3),
            PickupType::Missile =>             Some(4),
            PickupType::ScanVisor =>           Some(5),
            PickupType::MorphBallBomb =>       Some(6),
            PickupType::PowerBomb =>           Some(7),
            PickupType::Flamethrower =>        Some(8),
            PickupType::ThermalVisor =>        Some(9),
            PickupType::ChargeBeam =>          Some(10),
            PickupType::SuperMissile =>        Some(11),
            PickupType::GrappleBeam =>         Some(12),
            PickupType::XRayVisor =>           Some(13),
            PickupType::IceSpreader =>         Some(14),
            PickupType::SpaceJumpBoots =>      Some(15),
            PickupType::MorphBall =>           Some(16),
            // PickupType::CombatVisor =>         Some(17),
            PickupType::BoostBall =>           Some(18),
            PickupType::SpiderBall =>          Some(19),
            // PickupType::PowerSuit =>           Some(20),
            PickupType::GravitySuit =>         Some(21),
            PickupType::VariaSuit =>           Some(22),
            PickupType::PhazonSuit =>          Some(23),
            PickupType::EnergyTank =>          Some(24),
            PickupType::UnknownItem1 =>        Some(25),
            PickupType::HealthRefill =>        Some(26),
            PickupType::UnknownItem2 =>        Some(27),
            PickupType::Wavebuster =>          Some(28),
            PickupType::ArtifactOfTruth =>     Some(29),
            PickupType::ArtifactOfStrength =>  Some(30),
            PickupType::ArtifactOfElder =>     Some(31),
            PickupType::ArtifactOfWild =>      Some(32),
            PickupType::ArtifactOfLifegiver => Some(33),
            PickupType::ArtifactOfWarrior =>   Some(34),
            PickupType::ArtifactOfChozo =>     Some(35),
            PickupType::ArtifactOfNature =>    Some(36),
            PickupType::ArtifactOfSun =>       Some(37),
            PickupType::ArtifactOfWorld =>     Some(38),
            PickupType::ArtifactOfSpirit =>    Some(39),
            PickupType::ArtifactOfNewborn =>   Some(40),
            _ => None,
        }
    }

    pub fn iter() -> impl Iterator<Item = PickupType>
    {
        [
            PickupType::Missile,
            PickupType::EnergyTank,
            PickupType::ThermalVisor,
            PickupType::XRayVisor,
            PickupType::VariaSuit,
            PickupType::GravitySuit,
            PickupType::PhazonSuit,
            PickupType::MorphBall,
            PickupType::BoostBall,
            PickupType::SpiderBall,
            PickupType::MorphBallBomb,
            PickupType::PowerBombExpansion,
            PickupType::PowerBomb,
            PickupType::ChargeBeam,
            PickupType::SpaceJumpBoots,
            PickupType::GrappleBeam,
            PickupType::SuperMissile,
            PickupType::Wavebuster,
            PickupType::IceSpreader,
            PickupType::Flamethrower,
            PickupType::WaveBeam,
            PickupType::IceBeam,
            PickupType::PlasmaBeam,
            PickupType::ArtifactOfLifegiver,
            PickupType::ArtifactOfWild,
            PickupType::ArtifactOfWorld,
            PickupType::ArtifactOfSun,
            PickupType::ArtifactOfElder,
            PickupType::ArtifactOfSpirit,
            PickupType::ArtifactOfTruth,
            PickupType::ArtifactOfChozo,
            PickupType::ArtifactOfWarrior,
            PickupType::ArtifactOfNewborn,
            PickupType::ArtifactOfNature,
            PickupType::ArtifactOfStrength,
            PickupType::Nothing,
            PickupType::ScanVisor,
            PickupType::ShinyMissile,
            PickupType::PowerBeam,
            PickupType::UnknownItem1,
            PickupType::UnknownItem2,
            PickupType::HealthRefill,
            PickupType::MissileRefill,
            PickupType::PowerBombRefill,
        ].iter().map(|i| *i)
    }

    pub fn from_str(string: &str) -> Self {
        for i in PickupType::iter() {
            if i.name().to_string().to_lowercase().trim() == string.to_lowercase().trim() {
                return i;
            }
        }

        panic!("Unknown Item Type - {}", string);
    }
}

/// Lookup a pre-computed AABB for a pickup's CMDL
pub fn aabb_for_pickup_cmdl(id: structs::ResId<structs::res_id::CMDL>) -> Option<[f32; 6]>
{
    let id: u32 = id.into();
    // The aabb array is sorted, so we can binary search.
    if let Ok(idx) = PICKUP_CMDL_AABBS.binary_search_by_key(&id, |&(k, _)| k) {
        // The arrays contents are stored as u32s to reduce percision loss from
        // being converted to/from decimal literals. We use mem::transmute to
        // convert the u32s into f32s.
        Some(unsafe { mem::transmute(PICKUP_CMDL_AABBS[idx].1) })
    } else {
        None
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PickupLocation
{
    pub location: ScriptObjectLocation,
    pub attainment_audio: ScriptObjectLocation,
    pub hudmemo: ScriptObjectLocation,
    pub post_pickup_relay_connections: &'static [Connection],
    pub position: [f32;3],
}

#[derive(Clone, Copy, Debug)]
pub struct DoorLocation
{
    pub door_location: ScriptObjectLocation,
    pub door_force_location: ScriptObjectLocation,
    pub door_shield_location: Option<ScriptObjectLocation>,
    pub dock_number: Option<u32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct ScriptObjectLocation
{
    pub layer: u32,
    pub instance_id: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct RoomInfo
{
    pub room_id: ResId<res_id::MREA>,
    pub name: &'static str,
    pub name_id: ResId<res_id::STRG>,
    pub mapa_id: ResId<res_id::MAPA>,
    pub pickup_locations: &'static [PickupLocation],
    pub door_locations: &'static [DoorLocation],
    pub objects_to_remove: &'static [ObjectsToRemove],
}

#[derive(Clone, Copy, Debug)]
pub struct ObjectsToRemove
{
    pub layer: u32,
    pub instance_ids: &'static [u32],
}

impl RoomInfo
{
    pub fn from_str(string: &str) -> Self
    {
        for (_, rooms) in ROOM_INFO.iter() {
            for room_info in rooms.iter() {
                if room_info.name == string {
                    return *room_info;
                }
            }
        }

        panic!("Could not find room {}", string)
    }
}

include!("pickup_meta.rs.in");
