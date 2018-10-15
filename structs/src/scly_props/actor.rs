
use SclyPropertyData;
use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use scly_props::structs::{ActorParameters, AncsProp, DamageVulnerability, HealthInfo};


auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Debug, Clone)]
    pub struct Actor<'a>
    {
        #[expect = 24]
        prop_count: u32,

        name: CStr<'a>,

        position: GenericArray<f32, U3>,
        rotation: GenericArray<f32, U3>,
        scale: GenericArray<f32, U3>,
        unknown0: GenericArray<f32, U3>,// hitbox?
        scan_offset: GenericArray<f32, U3>,

        unknown1: f32,
        unknown2: f32,

        health_info: HealthInfo,
        damage_vulnerability: DamageVulnerability,

        cmdl: u32,
        ancs: AncsProp,
        actor_params: ActorParameters,

        looping: u8,
        snow: u8,
        solid: u8,
        camera_passthrough: u8,
        active: u8,
        unknown8: u32,
        unknown9: f32,
        unknown10: u8,
        unknown11: u8,
        unknown12: u8,
        unknown13: u8,
    }
}

impl<'a> SclyPropertyData for Actor<'a>
{
    const OBJECT_TYPE: u8 = 0x0;
}
