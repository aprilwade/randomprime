
use SclyPropertyData;
use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use scly_props::structs::{ActorParameters, AncsProp, DamageVulnerability, HealthInfo,
                          PlayerActorParams};


auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Debug, Clone)]
    pub struct PlayerActor<'a>
    {
        #[expect = 19]
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

        loop_animation: u8,
        unknown3: u8,
        disable_movement: u8,
        active: u8,
        player_actor_params: PlayerActorParams,
        unknown8: u32,
    }
}

impl<'a> SclyPropertyData for PlayerActor<'a>
{
    fn object_type() -> u8
    {
        0x4c
    }
}
