use auto_struct_macros::auto_struct;

use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct ActorParameters
{
    #[auto_struct(expect = 14)]
    prop_count: u32,
    pub light_params: LightParameters,
    pub scan_params: ScannableParameters,

    pub xray_cmdl: u32,
    pub xray_cskr: u32,

    pub thermal_cmdl: u32,
    pub thermal_cskr: u32,

    pub unknown0: u8,
    pub unknown1: f32,
    pub unknown2: f32,

    pub visor_params: VisorParameters,

    pub enable_thermal_heat: u8,
    pub unknown3: u8,
    pub unknown4: u8,
    pub unknown5: f32,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct AncsProp
{
    pub file_id: u32,
    pub node_index: u32,
    pub unknown: u32,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct LightParameters
{
    #[auto_struct(expect = 14)]
    prop_count: u32,

    pub unknown0: u8,
    pub unknown1: f32,
    pub shadow_tessellation: u32,
    pub unknown2: f32,
    pub unknown3: f32,
    pub color: GenericArray<f32, U4>, // RGBA
    pub unknown4: u8,
    pub world_lighting: u32,
    pub light_recalculation: u32,
    pub unknown5: GenericArray<f32, U3>,
    pub unknown6: u32,
    pub unknown7: u32,
    pub unknown8: u8,
    pub light_layer_id: u32,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct ScannableParameters
{
    #[auto_struct(expect = 1)]
    prop_count: u32,
    pub scan: u32,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct VisorParameters
{
    #[auto_struct(expect = 3)]
    prop_count: u32,
    pub unknown0: u8,
    pub target_passthrough: u8,
    pub unknown2: u32,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct DamageInfo
{
    #[auto_struct(expect = 4)]
    prop_count: u32,
    pub weapon_type: u32,
    pub damage: f32,
    pub radius: f32,
    pub knockback_power: f32,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct DamageVulnerability
{
    #[auto_struct(expect = 18)]
    prop_count: u32,

    pub power: u32,
    pub ice: u32,
    pub wave: u32,
    pub plasma: u32,
    pub bomb: u32,
    pub power_bomb: u32,
    pub missile: u32,
    pub boost_ball: u32,
    pub phazon: u32,

    pub enemy_weapon0: u32,
    pub enemy_weapon1: u32,
    pub enemy_weapon2: u32,
    pub enemy_weapon3: u32,

    pub unknown_weapon0: u32,
    pub unknown_weapon1: u32,
    pub unknown_weapon2: u32,

    pub charged_beams: ChargedBeams,
    pub beam_combos: BeamCombos,

}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct ChargedBeams
{
    #[auto_struct(expect = 5)]
    prop_count: u32,

    pub power: u32,
    pub ice: u32,
    pub wave: u32,
    pub plasma: u32,
    pub phazon: u32,
}


#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct BeamCombos
{
    #[auto_struct(expect = 5)]
    prop_count: u32,

    pub power: u32,
    pub ice: u32,
    pub wave: u32,
    pub plasma: u32,
    pub phazon: u32,
}


#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct HealthInfo
{
    #[auto_struct(expect = 2)]
    prop_count: u32,

    pub health: f32,
    pub knockback_resistance: f32,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct PlayerActorParams
{
    #[auto_struct(expect = 5)]
    prop_count: u32,

    pub unknown0: u8,
    pub unknown1: u8,
    pub unknown2: u8,
    pub unknown3: u8,
    pub unknown4: u8,
}
