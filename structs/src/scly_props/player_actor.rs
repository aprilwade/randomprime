use auto_struct_macros::auto_struct;

use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use crate::{ResId, SclyPropertyData};
use crate::res_id::*;
use crate::scly_props::structs::{
    ActorParameters, AncsProp, DamageVulnerability, HealthInfo
};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct PlayerActor<'r>
{
    #[auto_struct(expect = 19)]
    prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,
    pub unknown0: GenericArray<f32, U3>,// hitbox?
    pub scan_offset: GenericArray<f32, U3>,

    pub unknown1: f32,
    pub unknown2: f32,

    pub health_info: HealthInfo,
    pub damage_vulnerability: DamageVulnerability,

    pub cmdl: ResId<CMDL>,
    pub ancs: AncsProp,
    pub actor_params: ActorParameters,

    pub loop_animation: u8,
    pub unknown3: u8,
    pub disable_movement: u8,
    pub active: u8,
    pub player_actor_params: PlayerActorParams,
    pub unknown8: u32,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct PlayerActorParams
{
    #[auto_struct(derive = 5 + unknown5.is_some() as u32)]
    prop_count: u32,

    pub unknown0: u8,
    pub unknown1: u8,
    pub unknown2: u8,
    pub unknown3: u8,
    pub unknown4: u8,
    #[auto_struct(init = if prop_count == 6 { Some(()) } else { None })]
    pub unknown5: Option<u8>,
}

impl<'r> SclyPropertyData for PlayerActor<'r>
{
    const OBJECT_TYPE: u8 = 0x4c;
}
