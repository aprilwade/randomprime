use auto_struct_macros::auto_struct;
use reader_writer::{CStr, FourCC, IteratorArray, LazyArray, Readable, Reader, RoArray,
                    RoArrayIter, Writable};
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;

use crate::ResId;
use crate::res_id::*;

use std::io;
use std::iter::Peekable;

#[auto_struct(Readable, Writable)]
#[derive(Clone, Debug)]
pub struct Mlvl<'r>
{
    #[auto_struct(expect = 0xDEAFBABE)]
    magic: u32,

    #[auto_struct(expect = 0x11)]
    version: u32,

    pub world_name_strg: ResId<STRG>,
    pub world_savw: ResId<SAVW>,
    pub default_skybox_cmdl: ResId<CMDL>,

    #[auto_struct(derive = memory_relay_conns.len() as u32)]
    memory_relay_conn_count: u32,
    #[auto_struct(init = (memory_relay_conn_count as usize, ()))]
    pub memory_relay_conns: LazyArray<'r, MemoryRelayConn>,

    #[auto_struct(derive = areas.len() as u32)]
    area_count: u32,
    #[auto_struct(expect = 1)]
    unknown0: u32,
    #[auto_struct(init = (area_count as usize, ()))]
    pub areas: LazyArray<'r, Area<'r>>,

    pub world_map_mapw: u32,
    #[auto_struct(expect = 0)]
    unknown1: u8,

    #[auto_struct(expect = 0)]
    script_instance_count: u32,

    #[auto_struct(derive = audio_groups.len() as u32)]
    audio_group_count: u32,
    #[auto_struct(init = (audio_group_count as usize, ()))]
    pub audio_groups: RoArray<'r, AudioGroup>,

    #[auto_struct(expect = 0)]
    unknown2: u8,

    #[auto_struct(expect = areas.len() as u32)]
    area_count2: u32,
    #[auto_struct(init = (area_count as usize, ()))]
    pub area_layer_flags: LazyArray<'r, AreaLayerFlags>,

    // TODO: Could this be done lazily? Does it matter? We're basically always going
    //       to be modifying this structure, so maybe it would just be a waste?
    #[auto_struct(init = area_count)]
    pub area_layer_names: AreaLayerNames<'r>,

    #[auto_struct(pad_align = 32)]
    _pad: (),
}


#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Clone, Debug)]
pub struct MemoryRelayConn
{
    pub sender_id: u32,
    pub target_id: u32,
    pub message: u16,
    pub active: u8,
}

#[auto_struct(Readable, Writable)]
#[derive(Clone, Debug)]
pub struct Area<'r>
{
    pub area_name_strg: ResId<STRG>,
    pub area_transform: GenericArray<f32, U12>,
    pub area_bounding_box: GenericArray<f32, U6>,
    pub mrea: ResId<MREA>,

    pub internal_id: u32,

    pub attached_area_count: u32,
    #[auto_struct(init = (attached_area_count as usize, ()))]
    pub attached_areas: LazyArray<'r, u16>,

    // Not actually unknown, length of an array that's always empty...
    #[auto_struct(expect = 0)]
    _unused0: u32,

    pub dependencies: AreaDependencies<'r>,

    #[auto_struct(derive = docks.len() as u32)]
    dock_count: u32,
    #[auto_struct(init = (dock_count as usize, ()))]
    pub docks: LazyArray<'r, Dock<'r>>,
}

#[auto_struct(Readable, Writable)]
#[derive(Clone, Debug)]
pub struct AreaDependenciesInner<'r>
{
    #[auto_struct(derive = dependencies.len() as u32)]
    dependencies_count: u32,
    #[auto_struct(init = (dependencies_count as usize, ()))]
    pub dependencies: RoArray<'r, Dependency>,

    #[auto_struct(derive = dependency_offsets.len() as u32)]
    dependency_offsets_count: u32,
    #[auto_struct(init = (dependency_offsets_count as usize, ()))]
    pub dependency_offsets: RoArray<'r, u32>,
}

// Dependencies are implemented as multiple adjacent arrays which are differentiated
// by an offset array. This is difficult to model, so it uses hand-written reading/
// writing code.
#[derive(Clone, Debug)]
pub struct AreaDependencies<'r>
{
    pub deps: IteratorArray<'r, LazyArray<'r, Dependency>, LayerDepCountIter<'r>>
}

impl<'r> Readable<'r> for AreaDependencies<'r>
{
    type Args = ();
    fn read_from(reader: &mut Reader<'r>, (): ()) -> Self
    {
        let inner: AreaDependenciesInner = reader.read(());

        let mut data_start = inner.dependencies.data_start();
        let iter = LayerDepCountIter::new(inner);
        AreaDependencies { deps: data_start.read(iter), }
    }

    fn size(&self) -> usize
    {
        let deps_count: usize = self.deps.iter().map(|i| i.len()).sum();
        let s = u32::fixed_size().unwrap();
        s * (2 + self.deps.len()) + Dependency::fixed_size().unwrap() * deps_count
    }
}

impl<'r> Writable for AreaDependencies<'r>
{
    fn write_to<W: io::Write>(&self, writer: &mut W) -> io::Result<u64>
    {
        let mut sum = 0;
        let deps_count: u32 = self.deps.clone().iter().map(|i| i.len() as u32).sum();
        sum += deps_count.write_to(writer)?;
        sum += self.deps.write_to(writer)?;
        sum += (self.deps.len() as u32).write_to(writer)?;

        let mut offset_sum: u32 = 0;
        for array in self.deps.iter() {
            sum += offset_sum.write_to(writer)?;
            offset_sum += array.len() as u32;
        }
        Ok(sum)
    }
}

#[derive(Clone, Debug)]
pub struct LayerDepCountIter<'r>
{
    deps_len: u32,
    offsets_iter: Peekable<RoArrayIter<'r, u32>>,
}

impl<'r> LayerDepCountIter<'r>
{
    fn new(inner: AreaDependenciesInner<'r>) -> LayerDepCountIter<'r>
    {
        LayerDepCountIter {
            deps_len: inner.dependencies.len() as u32,
            offsets_iter: inner.dependency_offsets.iter().peekable(),
        }
    }
}

impl<'r> Iterator for LayerDepCountIter<'r>
{
    type Item = (usize, ());
    fn next(&mut self) -> Option<Self::Item>
    {
        let start = self.offsets_iter.next();
        let end = self.offsets_iter.peek().unwrap_or(&self.deps_len);
        start.map(|start| ((end - start) as usize, ()))
    }

    fn size_hint(&self) -> (usize, Option<usize>)
    {
        self.offsets_iter.size_hint()
    }
}

impl<'r> ExactSizeIterator for LayerDepCountIter<'r>
{
    fn len(&self) -> usize
    {
        self.offsets_iter.len()
    }
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Clone, Debug, PartialEq)]
pub struct Dependency
{
    pub asset_id: u32,
    pub asset_type: FourCC,
}
#[auto_struct(Readable, Writable)]
#[derive(Clone, Debug)]
pub struct Dock<'r>
{
    #[auto_struct(derive = connecting_docks.len() as u32 )]
    connecting_dock_count: u32,
    #[auto_struct(init = (connecting_dock_count as usize, ()))]
    pub connecting_docks: LazyArray<'r, DockConnection>,

    #[auto_struct(derive = dock_coordinates.len() as u32 )]
    dock_coordinate_count: u32,
    #[auto_struct(init = (dock_coordinate_count as usize, ()))]
    pub dock_coordinates: LazyArray<'r, GenericArray<f32, U3>>,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Clone, Debug)]
pub struct DockConnection
{
    pub array_index: u32,
    pub dock_index: u32,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Clone, Debug)]
pub struct AudioGroup
{
    pub group_id: u32,
    pub agsc: ResId<AGSC>,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Clone, Debug)]
pub struct AreaLayerFlags
{
    pub layer_count: u32,
    pub flags: u64,
}


#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Clone, Debug)]
struct AreaLayerNamesArgs<'r>
{
    #[auto_struct(derive = layer_names.len() as u32)]
    layer_names_count: u32,
    #[auto_struct(init = (layer_names_count as usize, ()))]
    pub layer_names: RoArray<'r, CStr<'r>>,

    #[auto_struct(derive = layer_names_offsets.len() as u32)]
    area_count: u32,
    #[auto_struct(init = (area_count as usize, ()))]
    pub layer_names_offsets: RoArray<'r, u32>,
}

// TODO: impl Deref(Mut)?
// TODO: If this were Vec<LazyArray> we could avoid some allocations
#[derive(Clone, Debug)]
pub struct AreaLayerNames<'r>(Vec<Vec<CStr<'r>>>);

impl<'r> AreaLayerNames<'r>
{
    pub fn new(offsets: RoArray<'r, u32>, names: RoArray<'r, CStr<'r>>) -> AreaLayerNames<'r>
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

    pub fn names_for_area(&self, area: usize) -> Option<&Vec<CStr<'r>>>
    {
        self.0.get(area)
    }

    pub fn mut_names_for_area(&mut self, area: usize) -> Option<&mut Vec<CStr<'r>>>
    {
        self.0.get_mut(area)
    }
}

impl<'r> Readable<'r> for AreaLayerNames<'r>
{
    type Args = u32;
    fn read_from(reader: &mut Reader<'r>, count: u32) -> Self
    {
        let args: AreaLayerNamesArgs = reader.read(());
        assert_eq!(args.layer_names_offsets.len(), count as usize);
        AreaLayerNames::new(args.layer_names_offsets, args.layer_names)
    }

    fn size(&self) -> usize
    {
        // TODO: It might be nice to cache this
        u32::fixed_size().unwrap() * (self.0.len() + 2) +
            self.0.iter().flat_map(|i| i).map(|s| s.to_bytes_with_nul().len()).sum::<usize>()
    }
}

impl<'r> Writable for AreaLayerNames<'r>
{
    fn write_to<W: io::Write>(&self, writer: &mut W) -> io::Result<u64>
    {
        let mut sum = 0;
        sum += self.0.iter().map(|area| area.len() as u32).sum::<u32>().write_to(writer)?;
        sum += self.0.write_to(writer)?;

        sum += (self.0.len() as u32).write_to(writer)?;

        let mut offset: u32 = 0;
        for area in &self.0 {
            sum += offset.write_to(writer)?;
            offset += area.len() as u32;
        }
        Ok(sum)
    }
}
