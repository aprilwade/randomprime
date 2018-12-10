
use SclyPropertyData;
use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use scly_props::structs::{ActorParameters, AncsProp};


auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Debug, Clone)]
    pub struct Pickup<'a>
    {
        #[expect = 18]
        prop_count: u32,

        name: CStr<'a>,

        position: GenericArray<f32, U3>,
        rotation: GenericArray<f32, U3>,
        scale: GenericArray<f32, U3>,
        hitbox: GenericArray<f32, U3>,
        scan_offset: GenericArray<f32, U3>,

        kind: u32,

        max_increase: u32,
        curr_increase: u32,

        drop_rate: f32,
        disappear_timer: f32,
        fade_in_timer: f32,

        cmdl: u32,
        ancs: AncsProp,
        actor_params: ActorParameters,

        active: u8,
        spawn_delay: f32,
        part: u32,
    }
}

impl<'a> SclyPropertyData for Pickup<'a>
{
    const OBJECT_TYPE: u8 = 0x11;
}
