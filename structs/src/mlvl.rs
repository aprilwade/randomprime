use reader_writer::{CStr, RoArray};
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;

auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Clone, Debug)]
    pub struct Mlvl<'a>
    {
        #[expect = 0xDEAFBABE]
        magic: u32,

        #[expect = 0x11]
        version: u32,

        world_name_strg: u32,
        world_savw: u32,
        default_skybox_cmdl: u32,

        #[derivable = memory_relays.len() as u32]
        memory_relay_count: u32,
        memory_relays: RoArray<'a, MemoryRelay> = (memory_relay_count as usize, ()),

        #[derivable = areas.len() as u32]
        area_count: u32,
        #[expect = 1]
        unknown0: u32,
        areas: RoArray<'a, Area<'a>> = (area_count as usize, ()),

        world_map_mapw: u32,
        #[expect = 0]
        unknown1: u8,

        #[expect = 0]
        script_instance_count: u32,

        #[derivable = audio_groups.len() as u32]
        audio_group_count: u32,
        audio_groups: RoArray<'a, AudioGroup> = (audio_group_count as usize, ()),

        #[expect = 0]
        unknown2: u8,

        #[expect = areas.len() as u32]
        area_count2: u32,
        area_layer_flags: RoArray<'a, AreaLayerFlags> = (area_count as usize, ()),

        #[derivable = layer_names.len() as u32]
        layer_names_count: u32,
        layer_names: RoArray<'a, CStr<'a>> = (layer_names_count as usize, ()),

        #[expect = areas.len() as u32]
        area_count3: u32,
        layer_names_offsets: RoArray<'a, u32> = (area_count as usize, ()),
    }
}


auto_struct! {
    #[auto_struct(Readable, Writable, FixedSize)]
    #[derive(Clone, Debug)]
    pub struct MemoryRelay
    {
        sender_id: u32,
        target_id: u32,
        message: u16,
        active: u8,
    }
}

auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Clone, Debug)]
    pub struct Area<'a>
    {
        area_name_strg: u32,
        area_transform: GenericArray<f32, U12>,
        area_bounding_box: GenericArray<f32, U6>,
        mrea: u32,

        internal_id: u32,

        attached_area_count: u32,
        attached_areas: RoArray<'a, u16> = (attached_area_count as usize, ()),

        // Not actually unknown, length of an array that's always empty...
        _unused0: u32,

        dependencies_count: u32,
        dependencies: RoArray<'a, Dependency> = (dependencies_count as usize, ()),
        dependency_offsets_count: u32,
        dependency_offsets: RoArray<'a, u32> = (dependency_offsets_count as usize, ()),

        dock_count: u32,
        docks: RoArray<'a, Dock<'a>> = (dock_count as usize, ()),
    }
}

auto_struct! {
    #[auto_struct(Readable, Writable, FixedSize)]
    #[derive(Clone, Debug)]
    pub struct Dependency
    {
        asset_id: u32,
        asset_type: u32,
    }
}
auto_struct! {
    #[auto_struct(Readable, Writable)]
    #[derive(Clone, Debug)]
    pub struct Dock<'a>
    {
        connecting_dock_count: u32,
        connecting_docks: RoArray<'a, DockConnection> = (connecting_dock_count as usize, ()),
        dock_coordinate_count: u32,
        dock_coordinates: RoArray<'a, GenericArray<f32, U3>> = (dock_coordinate_count as usize, ()),
    }
}

auto_struct! {
    #[auto_struct(Readable, Writable, FixedSize)]
    #[derive(Clone, Debug)]
    pub struct DockConnection
    {
        array_index: u32,
        dock_index: u32,
    }
}

auto_struct! {
    #[auto_struct(Readable, Writable, FixedSize)]
    #[derive(Clone, Debug)]
    pub struct AudioGroup
    {
        group_id: u32,
        agsc: u32,
    }
}

auto_struct! {
    #[auto_struct(Readable, Writable, FixedSize)]
    #[derive(Clone, Debug)]
    pub struct AreaLayerFlags
    {
        layer_count: u32,
        flags: u64,
    }
}
