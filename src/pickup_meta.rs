use std::mem;

use serde::Deserialize;

use reader_writer::{FourCC, Reader};
use structs::{Connection, ConnectionMsg, ConnectionState, Pickup, ResId, res_id};

use crate::custom_assets::custom_asset_ids;

/**
 * Pickup kind as defined by the game engine
 */
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PickupType
{
    PowerBeam = 0,
    IceBeam,
    WaveBeam,
    PlasmaBeam,
    Missile,
    ScanVisor,
    MorphBallBomb,
    PowerBomb,
    Flamethrower,
    ThermalVisor,
    ChargeBeam,
    SuperMissile,
    GrappleBeam,
    XRayVisor,
    IceSpreader,
    SpaceJumpBoots,
    MorphBall,
    CombatVisor,
    BoostBall,
    SpiderBall,
    PowerSuit,
    GravitySuit,
    VariaSuit,
    PhazonSuit,
    EnergyTank,
    UnknownItem1,
    HealthRefill,
    UnknownItem2,
    Wavebuster,
    ArtifactOfTruth,
    ArtifactOfStrength,
    ArtifactOfElder,
    ArtifactOfWild,
    ArtifactOfLifegiver,
    ArtifactOfWarrior,
    ArtifactOfChozo,
    ArtifactOfNature,
    ArtifactOfSun,
    ArtifactOfWorld,
    ArtifactOfSpirit,
    ArtifactOfNewborn,
    Nothing,
}

impl PickupType
{
    pub fn name(&self) -> &'static str
    {
        match self {
            PickupType::PowerBeam => "Power Beam",
            PickupType::IceBeam => "Ice Beam",
            PickupType::WaveBeam => "Wave Beam",
            PickupType::PlasmaBeam => "Plasma Beam",
            PickupType::Missile => "Missile",
            PickupType::ScanVisor => "Scan Visor",
            PickupType::MorphBallBomb => "Morph Ball Bomb",
            PickupType::PowerBomb => "Power Bomb",
            PickupType::Flamethrower => "Flamethrower",
            PickupType::ThermalVisor => "Thermal Visor",
            PickupType::ChargeBeam => "Charge Beam",
            PickupType::SuperMissile => "Super Missile",
            PickupType::GrappleBeam => "Grapple Beam",
            PickupType::XRayVisor => "X-Ray Visor",
            PickupType::IceSpreader => "Ice Spreader",
            PickupType::SpaceJumpBoots => "Space Jump Boots",
            PickupType::MorphBall => "Morph Ball",
            PickupType::CombatVisor => "Combat Visor",
            PickupType::BoostBall => "Boost Ball",
            PickupType::SpiderBall => "Spider Ball",
            PickupType::PowerSuit => "Power Suit",
            PickupType::GravitySuit => "Gravity Suit",
            PickupType::VariaSuit => "Varia Suit",
            PickupType::PhazonSuit => "Phazon Suit",
            PickupType::EnergyTank => "Energy Tank",
            PickupType::UnknownItem1 => "Unknown Item 1",
            PickupType::HealthRefill => "Health Refill",
            PickupType::UnknownItem2 => "Unknown Item 2",
            PickupType::Wavebuster => "Wavebuster",
            PickupType::ArtifactOfTruth => "Artifact Of Truth",
            PickupType::ArtifactOfStrength => "Artifact Of Strength",
            PickupType::ArtifactOfElder => "Artifact Of Elder",
            PickupType::ArtifactOfWild => "Artifact Of Wild",
            PickupType::ArtifactOfLifegiver => "Artifact Of Lifegiver",
            PickupType::ArtifactOfWarrior => "Artifact Of Warrior",
            PickupType::ArtifactOfChozo => "Artifact Of Chozo",
            PickupType::ArtifactOfNature => "Artifact Of Nature",
            PickupType::ArtifactOfSun => "Artifact Of Sun",
            PickupType::ArtifactOfWorld => "Artifact Of World",
            PickupType::ArtifactOfSpirit => "Artifact Of Spirit",
            PickupType::ArtifactOfNewborn => "Artifact Of Newborn",
            PickupType::Nothing => "Nothing",
        }
    }

    pub fn iter() -> impl Iterator<Item = PickupType>
    {
        [
            PickupType::PowerBeam,
            PickupType::IceBeam,
            PickupType::WaveBeam,
            PickupType::PlasmaBeam,
            PickupType::Missile,
            PickupType::ScanVisor,
            PickupType::MorphBallBomb,
            PickupType::PowerBomb,
            PickupType::Flamethrower,
            PickupType::ThermalVisor,
            PickupType::ChargeBeam,
            PickupType::SuperMissile,
            PickupType::GrappleBeam,
            PickupType::XRayVisor,
            PickupType::IceSpreader,
            PickupType::SpaceJumpBoots,
            PickupType::MorphBall,
            PickupType::CombatVisor,
            PickupType::BoostBall,
            PickupType::SpiderBall,
            PickupType::PowerSuit,
            PickupType::GravitySuit,
            PickupType::VariaSuit,
            PickupType::PhazonSuit,
            PickupType::EnergyTank,
            PickupType::UnknownItem1,
            PickupType::HealthRefill,
            PickupType::UnknownItem2,
            PickupType::Wavebuster,
            PickupType::ArtifactOfTruth,
            PickupType::ArtifactOfStrength,
            PickupType::ArtifactOfElder,
            PickupType::ArtifactOfWild,
            PickupType::ArtifactOfLifegiver,
            PickupType::ArtifactOfWarrior,
            PickupType::ArtifactOfChozo,
            PickupType::ArtifactOfNature,
            PickupType::ArtifactOfSun,
            PickupType::ArtifactOfWorld,
            PickupType::ArtifactOfSpirit,
            PickupType::ArtifactOfNewborn,
            PickupType::Nothing,
        ].iter().map(|i| *i)
    }

    pub fn kind(&self) -> u32
    {
        *self as u32
    }

    pub fn from_str(string: &str) -> Self {
        for i in PickupType::iter() {
            if i.name().to_string().to_lowercase().trim() == string.to_lowercase().trim() {
                return i;
            }
        }

        panic!("Unknown Pickup Type - {}", string);
    }

    /**
     * asset IDs of default text (e.g. "Power Bombs Aquired")
     */
    pub fn scan_strg(&self) -> ResId<res_id::STRG> {
        ResId::<res_id::STRG>::new(custom_asset_ids::DEFAULT_PICKUP_SCAN_STRGS.to_u32() + self.kind())
    }

    pub fn scan(&self) -> ResId<res_id::SCAN> {
        ResId::<res_id::SCAN>::new(custom_asset_ids::DEFAULT_PICKUP_SCANS.to_u32() + self.kind())
    }

    pub fn hudmemo_strg(&self) -> ResId<res_id::STRG> {
        ResId::<res_id::STRG>::new(custom_asset_ids::DEFAULT_PICKUP_HUDMEMO_STRGS.to_u32() + self.kind())
    }
}

/* CMDL which exist in the vanilla game, or are custom-made for randomprime */
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PickupModel
{
    Missile,
    EnergyTank,
    Visor,
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
    HealthRefill,
    MissileRefill,
    PowerBombRefill,
    ShinyMissile,
}

impl PickupModel
{
    pub fn name(&self) -> &'static str
    {
        match self {
            PickupModel::Missile =>             "Missile",
            PickupModel::EnergyTank =>          "Energy Tank",
            PickupModel::Visor =>               "Visor",
            PickupModel::VariaSuit =>           "Varia Suit",
            PickupModel::GravitySuit =>         "Gravity Suit",
            PickupModel::PhazonSuit =>          "Phazon Suit",
            PickupModel::MorphBall =>           "Morph Ball",
            PickupModel::BoostBall =>           "Boost Ball",
            PickupModel::SpiderBall =>          "Spider Ball",
            PickupModel::MorphBallBomb =>       "Morph Ball Bomb",
            PickupModel::PowerBombExpansion =>  "Power Bomb Expansion",
            PickupModel::PowerBomb =>           "Power Bomb",
            PickupModel::ChargeBeam =>          "Charge Beam",
            PickupModel::SpaceJumpBoots =>      "Space Jump Boots",
            PickupModel::GrappleBeam =>         "Grapple Beam",
            PickupModel::SuperMissile =>        "Super Missile",
            PickupModel::Wavebuster =>          "Wavebuster",
            PickupModel::IceSpreader =>         "Ice Spreader",
            PickupModel::Flamethrower =>        "Flamethrower",
            PickupModel::WaveBeam =>            "Wave Beam",
            PickupModel::IceBeam =>             "Ice Beam",
            PickupModel::PlasmaBeam =>          "Plasma Beam",
            PickupModel::ArtifactOfLifegiver => "Artifact of Lifegiver",
            PickupModel::ArtifactOfWild =>      "Artifact of Wild",
            PickupModel::ArtifactOfWorld =>     "Artifact of World",
            PickupModel::ArtifactOfSun =>       "Artifact of Sun",
            PickupModel::ArtifactOfElder =>     "Artifact of Elder",
            PickupModel::ArtifactOfSpirit =>    "Artifact of Spirit",
            PickupModel::ArtifactOfTruth =>     "Artifact of Truth",
            PickupModel::ArtifactOfChozo =>     "Artifact of Chozo",
            PickupModel::ArtifactOfWarrior =>   "Artifact of Warrior",
            PickupModel::ArtifactOfNewborn =>   "Artifact of Newborn",
            PickupModel::ArtifactOfNature =>    "Artifact of Nature",
            PickupModel::ArtifactOfStrength =>  "Artifact of Strength",
            PickupModel::Nothing =>             "Nothing",
            PickupModel::HealthRefill =>        "Health Refill",
            PickupModel::MissileRefill =>       "Missile Refill",
            PickupModel::PowerBombRefill =>     "Power Bomb Refill",
            PickupModel::ShinyMissile =>        "Shiny Missile",
        }
    }

    pub fn pickup_data<'a>(&self) -> Pickup
    {
        let mut pickup: Pickup = Reader::new(self.raw_pickup_data()).read(());
        if self.name() == PickupModel::Nothing.name() {
            pickup.scale[0] = 1.0;
            pickup.scale[1] = 1.0;
            pickup.scale[2] = 1.0;
        }
        pickup
    }

    pub fn iter() -> impl Iterator<Item = PickupModel>
    {
        [
            PickupModel::Missile, 
            PickupModel::EnergyTank, 
            PickupModel::Visor, 
            PickupModel::VariaSuit, 
            PickupModel::GravitySuit, 
            PickupModel::PhazonSuit, 
            PickupModel::MorphBall, 
            PickupModel::BoostBall, 
            PickupModel::SpiderBall, 
            PickupModel::MorphBallBomb, 
            PickupModel::PowerBombExpansion, 
            PickupModel::PowerBomb, 
            PickupModel::ChargeBeam, 
            PickupModel::SpaceJumpBoots, 
            PickupModel::GrappleBeam, 
            PickupModel::SuperMissile, 
            PickupModel::Wavebuster, 
            PickupModel::IceSpreader, 
            PickupModel::Flamethrower, 
            PickupModel::WaveBeam, 
            PickupModel::IceBeam, 
            PickupModel::PlasmaBeam, 
            PickupModel::ArtifactOfLifegiver, 
            PickupModel::ArtifactOfWild, 
            PickupModel::ArtifactOfWorld, 
            PickupModel::ArtifactOfSun, 
            PickupModel::ArtifactOfElder, 
            PickupModel::ArtifactOfSpirit, 
            PickupModel::ArtifactOfTruth, 
            PickupModel::ArtifactOfChozo, 
            PickupModel::ArtifactOfWarrior, 
            PickupModel::ArtifactOfNewborn, 
            PickupModel::ArtifactOfNature, 
            PickupModel::ArtifactOfStrength, 
            PickupModel::Nothing, 
            PickupModel::HealthRefill, 
            PickupModel::MissileRefill, 
            PickupModel::PowerBombRefill, 
            PickupModel::ShinyMissile,
        ].iter().map(|i| *i)
    }

    pub fn from_str(string: &str) -> Option<Self> {
        let string = string.to_lowercase();
        let string = string.trim();
        for i in PickupModel::iter() {
            if i.name().to_string().to_lowercase().trim() == string {
                return Some(i);
            }
        }

        // Deprecated Maping
        if vec!["combat visor", "scan visor", "x-ray visor", "xray visor", "thermal visor", "combat", "scan", "xray", "thermal"]
            .contains(&string)
        {
            return Some(PickupModel::Visor);
        }

        None
    }

    /**
     * Used to determine default model if none is provided
     */
    pub fn from_type(pickup_type: PickupType) -> Self {
        match pickup_type {
            PickupType::PowerBeam           => PickupModel::Nothing,
            PickupType::IceBeam             => PickupModel::IceBeam,
            PickupType::WaveBeam            => PickupModel::WaveBeam,
            PickupType::PlasmaBeam          => PickupModel::PlasmaBeam,
            PickupType::Missile             => PickupModel::Missile,
            PickupType::ScanVisor           => PickupModel::Visor,
            PickupType::MorphBallBomb       => PickupModel::MorphBallBomb,
            PickupType::PowerBomb           => PickupModel::PowerBomb,
            PickupType::Flamethrower        => PickupModel::Flamethrower,
            PickupType::ThermalVisor        => PickupModel::Visor,
            PickupType::ChargeBeam          => PickupModel::ChargeBeam,
            PickupType::SuperMissile        => PickupModel::SuperMissile,
            PickupType::GrappleBeam         => PickupModel::GrappleBeam,
            PickupType::XRayVisor           => PickupModel::Visor,
            PickupType::IceSpreader         => PickupModel::IceSpreader,
            PickupType::SpaceJumpBoots      => PickupModel::SpaceJumpBoots,
            PickupType::MorphBall           => PickupModel::MorphBall,
            PickupType::CombatVisor         => PickupModel::Visor,
            PickupType::BoostBall           => PickupModel::BoostBall,
            PickupType::SpiderBall          => PickupModel::SpiderBall,
            PickupType::PowerSuit           => PickupModel::Nothing,
            PickupType::GravitySuit         => PickupModel::GravitySuit,
            PickupType::VariaSuit           => PickupModel::VariaSuit,
            PickupType::PhazonSuit          => PickupModel::PhazonSuit,
            PickupType::EnergyTank          => PickupModel::EnergyTank,
            PickupType::UnknownItem1        => PickupModel::Nothing,
            PickupType::HealthRefill        => PickupModel::HealthRefill,
            PickupType::UnknownItem2        => PickupModel::Nothing,
            PickupType::Wavebuster          => PickupModel::Wavebuster,
            PickupType::ArtifactOfTruth     => PickupModel::ArtifactOfTruth,
            PickupType::ArtifactOfStrength  => PickupModel::ArtifactOfStrength,
            PickupType::ArtifactOfElder     => PickupModel::ArtifactOfElder,
            PickupType::ArtifactOfWild      => PickupModel::ArtifactOfWild,
            PickupType::ArtifactOfLifegiver => PickupModel::ArtifactOfLifegiver,
            PickupType::ArtifactOfWarrior   => PickupModel::ArtifactOfWarrior,
            PickupType::ArtifactOfChozo     => PickupModel::ArtifactOfChozo,
            PickupType::ArtifactOfNature    => PickupModel::ArtifactOfNature,
            PickupType::ArtifactOfSun       => PickupModel::ArtifactOfSun,
            PickupType::ArtifactOfWorld     => PickupModel::ArtifactOfWorld,
            PickupType::ArtifactOfSpirit    => PickupModel::ArtifactOfSpirit,
            PickupType::ArtifactOfNewborn   => PickupModel::ArtifactOfNewborn,
            PickupType::Nothing             => PickupModel::Nothing,
        }
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
