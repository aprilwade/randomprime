use auto_struct_macros::auto_struct;

use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use crate::{ResId, SclyPropertyData};
use crate::res_id::*;
use crate::scly_props::structs::{ActorParameters, AncsProp, DamageVulnerability, HealthInfo};


#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Platform<'r>
{
    #[auto_struct(expect = 19)]
    prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,
    pub unknown0: GenericArray<f32, U3>,// hitbox?
    pub scan_offset: GenericArray<f32, U3>,

    pub cmdl: ResId<CMDL>,
    pub ancs: AncsProp,
    pub actor_params: ActorParameters,

    pub unknown1: f32,
    pub active: u8,

    pub dcln: ResId<DCLN>,

    pub health_info: HealthInfo,
    pub damage_vulnerability: DamageVulnerability,

    pub unknown3: u8,
    pub unknown4: f32,
    pub unknown5: u8,
    pub unknown6: u32,
    pub unknown7: u32,
}

impl<'r> SclyPropertyData for Platform<'r>
{
    const OBJECT_TYPE: u8 = 0x8;
}
