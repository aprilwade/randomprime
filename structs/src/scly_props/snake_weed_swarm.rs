use auto_struct_macros::auto_struct;

use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use crate::{SclyPropertyData};
use crate::scly_props::structs::{ActorParameters, AncsProp, DamageInfo};


#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct SnakeWeedSwarm<'r>
{
    #[auto_struct(expect = 25)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,

    pub active: u8,
    pub ancs: AncsProp,
    pub actor_params: ActorParameters,

    pub unknown0: f32,
    pub unknown1: f32,
    pub unknown2: f32,
    pub unknown3: f32,
    pub unknown4: f32,
    pub unknown5: f32,
    pub unknown6: f32,
    pub unknown7: f32,
    pub unknown8: f32,
    pub unknown9: f32,
    pub unknown10: f32,
    pub unknown11: f32,
    pub unknown12: f32,
    pub unknown13: f32,

    pub damage_info: DamageInfo,

    pub unknown14: f32,

    pub unknown15: u32,
    pub unknown16: u32,
    pub unknown17: u32,
}

impl<'r> SclyPropertyData for SnakeWeedSwarm<'r>
{
    const OBJECT_TYPE: u8 = 0x6D;
}
