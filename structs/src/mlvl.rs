use reader_writer::{CStr, LazyArray, Readable, Reader, RoArray, Writable};
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;

use std::io::Write;

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
        areas: LazyArray<'a, Area<'a>> = (area_count as usize, ()),

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
        area_layer_flags: LazyArray<'a, AreaLayerFlags> = (area_count as usize, ()),

        // TODO: Could this be done lazily? Does it matter? We're basically always going
        //       to be modifying this structure, so maybe it would just be a waste?
        area_layer_names: AreaLayerNames<'a> = area_count,
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


auto_struct! {
    #[auto_struct(Readable, Writable, FixedSize)]
    #[derive(Clone, Debug)]
    struct AreaLayerNamesArgs<'a>
    {
        #[derivable = layer_names.len() as u32]
        layer_names_count: u32,
        layer_names: RoArray<'a, CStr<'a>> = (layer_names_count as usize, ()),

        #[derivable = layer_names_offsets.len() as u32]
        area_count: u32,
        layer_names_offsets: RoArray<'a, u32> = (area_count as usize, ()),
    }
}

// TODO: impl Deref(Mut)?
// TODO: If this were Vec<LazyArray> we could avoid some allocations
#[derive(Clone, Debug)]
pub struct AreaLayerNames<'a>(Vec<Vec<CStr<'a>>>);

impl<'a> AreaLayerNames<'a>
{
    pub fn new(offsets: RoArray<'a, u32>, names: RoArray<'a, CStr<'a>>) -> AreaLayerNames<'a>
    {
        use std::iter::once;

        // XXX We're assuming offsets is ordered
        let mut names_vec = Vec::with_capacity(offsets.len());
        let mut offsets_iter = offsets.iter();
        let mut names_iter = names.iter();

        let mut last_offset = offsets_iter.next().unwrap();
        assert_eq!(last_offset, 0);
        for offset in offsets_iter.chain(once(names.len() as u32)) {
            let count = offset - last_offset;
            let mut v = Vec::with_capacity(count as usize);
            for _ in 0..count {
                v.push(names_iter.next().unwrap())
            }
            names_vec.push(v);
            last_offset = offset;
        }

        AreaLayerNames(names_vec)
    }

    pub fn names_for_area(&self, area: usize) -> Option<&Vec<CStr<'a>>>
    {
        self.0.get(area)
    }

    pub fn mut_names_for_area(&mut self, area: usize) -> Option<&mut Vec<CStr<'a>>>
    {
        self.0.get_mut(area)
    }
}

impl<'a> Readable<'a> for AreaLayerNames<'a>
{
    type Args = u32;
    fn read(mut reader: Reader<'a>, count: u32) -> (Self, Reader<'a>)
    {
        let args: AreaLayerNamesArgs = reader.read(());
        assert_eq!(args.layer_names_offsets.len(), count as usize);
        (AreaLayerNames::new(args.layer_names_offsets, args.layer_names), reader)
    }

    fn size(&self) -> usize
    {
        // TODO: It might be nice to cache this
        self.0.len() * u32::fixed_size().unwrap() + 
            self.0.iter().flat_map(|i| i).map(|s| s.to_bytes_with_nul().len()).sum::<usize>()
    }
}

impl<'a> Writable for AreaLayerNames<'a>
{
    fn write<W: Write>(&self, writer: &mut W)
    {
        (self.0.iter().map(|i| i.len()).sum::<usize>() as u32).write(writer);
        for name in self.0.iter().flat_map(|i| i) {
            name.write(writer);
        }

        (self.0.len() as u32).write(writer);

        let mut offset: u32 = 0;
        for area in &self.0 {
            offset.write(writer);
            offset += area.len() as u32;
        }
    }
}
