use auto_struct_macros::auto_struct;

use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use crate::{ResId, SclyPropertyData};
use crate::res_id::*;
use crate::scly_props::structs::{DamageVulnerability, HealthInfo, VisorParameters};


#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct DamageableTrigger<'r>
{
    #[auto_struct(expect = 12)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,
    pub health_info: HealthInfo,
    pub damage_vulnerability: DamageVulnerability,

    pub unknown0: u32,

    pub pattern_txtr0: ResId<TXTR>,
    pub pattern_txtr1: ResId<TXTR>,
    pub color_txtr: ResId<TXTR>,

    pub lock_on: u8,
    pub active: u8,

    pub visor_params: VisorParameters
}

impl<'r> SclyPropertyData for DamageableTrigger<'r>
{
    const OBJECT_TYPE: u8 = 0x1A;
}
