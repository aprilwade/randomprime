
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

        // 6 unknown parameters
        unknown0: GenericArray<u8, U21>,

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


