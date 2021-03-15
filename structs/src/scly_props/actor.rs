use auto_struct_macros::auto_struct;

use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use crate::{ResId, SclyPropertyData};
use crate::res_id:: *;
use crate::scly_props::structs::{ActorParameters, AncsProp, DamageVulnerability, HealthInfo};


#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Actor<'r>
{
    #[auto_struct(expect = 24)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,
    pub hitbox: GenericArray<f32, U3>,
    pub scan_offset: GenericArray<f32, U3>,

    pub unknown1: f32,
    pub unknown2: f32,

    pub health_info: HealthInfo,
    pub damage_vulnerability: DamageVulnerability,

    pub cmdl: ResId<CMDL>,
    pub ancs: AncsProp,
    pub actor_params: ActorParameters,

    pub looping: u8,
    pub snow: u8,
    pub solid: u8,
    pub camera_passthrough: u8,
    pub active: u8,
    pub unknown8: u32,
    pub unknown9: f32,
    pub unknown10: u8,
    pub unknown11: u8,
    pub unknown12: u8,
    pub unknown13: u8,
}

impl<'r> SclyPropertyData for Actor<'r>
{
    const OBJECT_TYPE: u8 = 0x0;
}
