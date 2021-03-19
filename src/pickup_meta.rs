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
            PickupType::ShinyMissile =>        "Shiny Missile",
        }
    }

    pub fn idx(&self) -> usize
    {
        match self {
            PickupType::Missile =>             0,
            PickupType::EnergyTank =>          1,
            PickupType::ThermalVisor =>        2,
            PickupType::XRayVisor =>           3,
            PickupType::VariaSuit =>           4,
            PickupType::GravitySuit =>         5,
            PickupType::PhazonSuit =>          6,
            PickupType::MorphBall =>           7,
            PickupType::BoostBall =>           8,
            PickupType::SpiderBall =>          9,
            PickupType::MorphBallBomb =>       10,
            PickupType::PowerBombExpansion =>  11,
            PickupType::PowerBomb =>           12,
            PickupType::ChargeBeam =>          13,
            PickupType::SpaceJumpBoots =>      14,
            PickupType::GrappleBeam =>         15,
            PickupType::SuperMissile =>        16,
            PickupType::Wavebuster =>          17,
            PickupType::IceSpreader =>         18,
            PickupType::Flamethrower =>        19,
            PickupType::WaveBeam =>            20,
            PickupType::IceBeam =>             21,
            PickupType::PlasmaBeam =>          22,
            PickupType::ArtifactOfLifegiver => 23,
            PickupType::ArtifactOfWild =>      24,
            PickupType::ArtifactOfWorld =>     25,
            PickupType::ArtifactOfSun =>       26,
            PickupType::ArtifactOfElder =>     27,
            PickupType::ArtifactOfSpirit =>    28,
            PickupType::ArtifactOfTruth =>     29,
            PickupType::ArtifactOfChozo =>     30,
            PickupType::ArtifactOfWarrior =>   31,
            PickupType::ArtifactOfNewborn =>   32,
            PickupType::ArtifactOfNature =>    33,
            PickupType::ArtifactOfStrength =>  34,
            PickupType::Nothing =>             35,
            PickupType::ScanVisor =>           36,
            PickupType::ShinyMissile =>        37,
        }
    }

    pub fn from_idx(idx: usize) -> Option<Self>
    {
        match idx {
            0  => Some(PickupType::Missile),
            1  => Some(PickupType::EnergyTank),
            2  => Some(PickupType::ThermalVisor),
            3  => Some(PickupType::XRayVisor),
            4  => Some(PickupType::VariaSuit),
            5  => Some(PickupType::GravitySuit),
            6  => Some(PickupType::PhazonSuit),
            7  => Some(PickupType::MorphBall),
            8  => Some(PickupType::BoostBall),
            9  => Some(PickupType::SpiderBall),
            10 => Some(PickupType::MorphBallBomb),
            11 => Some(PickupType::PowerBombExpansion),
            12 => Some(PickupType::PowerBomb),
            13 => Some(PickupType::ChargeBeam),
            14 => Some(PickupType::SpaceJumpBoots),
            15 => Some(PickupType::GrappleBeam),
            16 => Some(PickupType::SuperMissile),
            17 => Some(PickupType::Wavebuster),
            18 => Some(PickupType::IceSpreader),
            19 => Some(PickupType::Flamethrower),
            20 => Some(PickupType::WaveBeam),
            21 => Some(PickupType::IceBeam),
            22 => Some(PickupType::PlasmaBeam),
            23 => Some(PickupType::ArtifactOfLifegiver),
            24 => Some(PickupType::ArtifactOfWild),
            25 => Some(PickupType::ArtifactOfWorld),
            26 => Some(PickupType::ArtifactOfSun),
            27 => Some(PickupType::ArtifactOfElder),
            28 => Some(PickupType::ArtifactOfSpirit),
            29 => Some(PickupType::ArtifactOfTruth),
            30 => Some(PickupType::ArtifactOfChozo),
            31 => Some(PickupType::ArtifactOfWarrior),
            32 => Some(PickupType::ArtifactOfNewborn),
            33 => Some(PickupType::ArtifactOfNature),
            34 => Some(PickupType::ArtifactOfStrength),
            35 => Some(PickupType::Nothing),
            36 => Some(PickupType::ScanVisor),
            _ => None,
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
        ResId::new((start..end).nth(self.idx()).unwrap())
    }

    pub fn pickup_data<'a>(&self) -> &'a Pickup<'static>
    {
        &PickupTable::get()[*self]
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
        ].iter().map(|i| *i)
    }

    pub fn from_string(string: String) -> Self {
        for i in PickupType::iter() {
            if i.name().to_string().to_lowercase().trim() == string.to_lowercase().trim() {
                return i;
            }
        }

        panic!("Unknown Item Type - {}", string);
    }
}

struct PickupTable(Vec<structs::Pickup<'static>>);

impl PickupTable
{
    fn new() -> PickupTable
    {
        PickupTable(PickupType::iter()
            .map(|pt| Reader::new(pt.raw_pickup_data()).read(()))
            .collect())
    }

    fn get<'a>() -> &'a PickupTable
    {
        static mut CACHED: Option<PickupTable> = None;
        if unsafe { CACHED.is_none() } {
            let pmt = PickupTable::new();
            unsafe { CACHED = Some(pmt) };
        }
        unsafe { CACHED.as_ref().unwrap() }
    }
}

impl std::ops::Index<PickupType> for PickupTable
{
    type Output = structs::Pickup<'static>;
    fn index(&self, ptype: PickupType) -> &Self::Output
    {
        &self.0[ptype.idx()]
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
    pub post_pickup_relay_connections: &'static [Connection]
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

include!("pickup_meta.rs.in");
