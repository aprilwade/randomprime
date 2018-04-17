
use SclyPropertyData;
use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use scly_props::structs::{ActorParameters, AncsProp, DamageVulnerability, HealthInfo};


auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Debug, Clone)]
    pub struct Platform<'a>
    {
        #[expect = 19]
        prop_count: u32,

        name: CStr<'a>,

        position: GenericArray<f32, U3>,
        rotation: GenericArray<f32, U3>,
        scale: GenericArray<f32, U3>,
        unknown0: GenericArray<f32, U3>,// hitbox?
        scan_offset: GenericArray<f32, U3>,

        cmdl: u32,
        ancs: AncsProp,
        actor_params: ActorParameters,

        unknown1: f32,
        active: u8,

        dcln: u32,

        health_info: HealthInfo,
        damage_vulnerability: DamageVulnerability,

        unknown3: u8,
        unknown4: f32,
        unknown5: u8,
        unknown6: u32,
        unknown7: u32,
    }
}

impl<'a> SclyPropertyData for Platform<'a>
{
    fn object_type() -> u8
    {
        0x8
    }
}
