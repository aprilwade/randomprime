
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;

auto_struct! {
    #[auto_struct(Readable, Writable, FixedSize)]
    #[derive(Debug, Clone)]
    pub struct ActorParameters
    {
        #[expect = 14]
        prop_count: u32,
        light_params: LightParameters,
        scan_params: ScannableParameters,

        xray_cmdl: u32,
        xray_cskr: u32,

        // 6 unknown parameters
        unknown0: GenericArray<u8, U17>,

        visor_params: VisorParameters,

        // 4 unknown parameters
        unknown1: GenericArray<u8, U7>,
    }
}

auto_struct! {
    #[auto_struct(Readable, Writable, FixedSize)]
    #[derive(Debug, Clone)]
    pub struct AncsProp
    {
        file_id: u32,
        node_index: u32,
        unknown: u32,
    }
}

auto_struct! {
    #[auto_struct(Readable, Writable, FixedSize)]
    #[derive(Debug, Clone)]
    pub struct LightParameters
    {
        #[expect = 14]
        prop_count: u32,
        // Details left out for simplicity
        unknown: GenericArray<u8, U67>,
    }
}

auto_struct! {
    #[auto_struct(Readable, Writable, FixedSize)]
    #[derive(Debug, Clone)]
    pub struct ScannableParameters
    {
        #[expect = 1]
        prop_count: u32,
        scan: u32,
    }
}

auto_struct! {
    #[auto_struct(Readable, Writable, FixedSize)]
    #[derive(Debug, Clone)]
    pub struct VisorParameters
    {
        #[expect = 3]
        prop_count: u32,
        unknown0: u8,
        unknown1: u8,
        unknown2: u32,
    }
}

auto_struct! {
    #[auto_struct(Readable, Writable, FixedSize)]
    #[derive(Debug, Clone)]
    pub struct DamageInfo
    {
        #[expect = 4]
        prop_count: u32,
        weapon_type: u32,
        damage: f32,
        radius: f32,
        knockback_power: f32,
    }
}

auto_struct! {
    #[auto_struct(Readable, Writable, FixedSize)]
    #[derive(Debug, Clone)]
    pub struct DamageVulnerability
    {
        #[expect = 18]
        prop_count: u32,

        power: u32,
        ice: u32,
        wave: u32,
        plasma: u32,
        bomb: u32,
        power_bomb: u32,
        missile: u32,
        boost_ball: u32,
        phazon: u32,

        enemy_weapon0: u32,
        enemy_weapon1: u32,
        enemy_weapon2: u32,
        enemy_weapon3: u32,

        unknown_weapon0: u32,
        unknown_weapon1: u32,
        unknown_weapon2: u32,

        charged_beams: ChargedBeams,
        beam_combos: BeamCombos,

    }
}

auto_struct! {
    #[auto_struct(Readable, Writable, FixedSize)]
    #[derive(Debug, Clone)]
    pub struct ChargedBeams
    {
        #[expect = 5]
        prop_count: u32,

        power: u32,
        ice: u32,
        wave: u32,
        plasma: u32,
        phazon: u32,
    }
}


auto_struct! {
    #[auto_struct(Readable, Writable, FixedSize)]
    #[derive(Debug, Clone)]
    pub struct BeamCombos
    {
        #[expect = 5]
        prop_count: u32,

        power: u32,
        ice: u32,
        wave: u32,
        plasma: u32,
        phazon: u32,
    }
}


auto_struct! {
    #[auto_struct(Readable, Writable, FixedSize)]
    #[derive(Debug, Clone)]
    pub struct HealthInfo
    {
        #[expect = 2]
        prop_count: u32,

        health: f32,
        knockback_resistance: f32,
    }
}

auto_struct! {
    #[auto_struct(Readable, Writable, FixedSize)]
    #[derive(Debug, Clone)]
    pub struct PlayerActorParams
    {
        #[expect = 5]
        prop_count: u32,

        unknown0: u8,
        unknown1: u8,
        unknown2: u8,
        unknown3: u8,
        unknown4: u8,
    }
}
