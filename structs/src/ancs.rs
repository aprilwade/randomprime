use auto_struct_macros::auto_struct;
use crate::ResId;
use crate::res_id:: *;

use reader_writer::{
    CStr, FourCC, LazyArray, IteratorArray, Readable, Reader, RoArray, Uncached, RoArrayIter,
    Writable,
};
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;

use std::io;

fn bool_to_opt(b: bool) -> Option<()>
{
    if b {
        Some(())
    } else {
        None
    }
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Ancs<'r>
{
    #[auto_struct(expect = 1)]
    version: u16,

    pub char_set: CharacterSet<'r>,
    pub anim_set: AnimationSet<'r>,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct CharacterSet<'r>
{
    #[auto_struct(expect = 1)]
    version: u16,

    #[auto_struct(derive = char_info.len() as u32)]
    pub char_info_count: u32,
    #[auto_struct(init = (char_info_count as usize, ()))]
    pub char_info: LazyArray<'r, CharacterInfo<'r>>,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct CharacterInfo<'r>
{
    pub id: u32,

    pub info_type_count: u16,

    pub name: CStr<'r>,

    pub cmdl: ResId<CMDL>,
    pub cskr: ResId<CSKR>,
    pub cinf: ResId<CINF>,

    pub animation_count: u32,
    #[auto_struct(init = (animation_count as usize, info_type_count))]
    pub animation_names: RoArray<'r, AnimationName<'r>>,

    pub pas_database: PasDatabase<'r>,
    #[auto_struct(init = info_type_count)]
    pub particles: ParticleResData<'r>,

    pub unknown0: u32,
    #[auto_struct(init = bool_to_opt(info_type_count > 9))]
    pub unknown1: Option<u32>,
    #[auto_struct(init = bool_to_opt(info_type_count > 9))]
    pub unknown2: Option<u32>,

    #[auto_struct(init = bool_to_opt(info_type_count > 1))]
    pub animation_aabb_count: Option<u32>,
    #[auto_struct(init = animation_aabb_count.map(|i| (i as usize, ())))]
    pub animation_aabbs: Option<RoArray<'r, AnimationAABB<'r>>>,

    #[auto_struct(init = bool_to_opt(info_type_count > 1))]
    pub effect_count: Option<u32>,
    #[auto_struct(init = effect_count.map(|i| (i as usize, ())))]
    pub effects: Option<RoArray<'r, Effect<'r>>>,

    #[auto_struct(init = bool_to_opt(info_type_count > 3))]
    pub overlay_cmdl: Option<ResId<CMDL>>,
    #[auto_struct(init = bool_to_opt(info_type_count > 3))]
    pub overlay_cskr: Option<ResId<CSKR>>,

    #[auto_struct(init = bool_to_opt(info_type_count > 4))]
    pub animation_index_count: Option<u32>,
    #[auto_struct(init = animation_index_count.map(|i| (i as usize, ())))]
    pub animation_indices: Option<RoArray<'r, u32>>,

    #[auto_struct(init = bool_to_opt(info_type_count > 9))]
    pub unknown3: Option<u32>,
    #[auto_struct(init = bool_to_opt(info_type_count > 9))]
    pub unknown4: Option<u8>,

    #[auto_struct(init = bool_to_opt(info_type_count > 9))]
    pub animation_indexed_aabb_count: Option<u32>,
    #[auto_struct(init = animation_indexed_aabb_count.map(|i| (i as usize, ())))]
    pub animation_indexed_aabbs: Option<RoArray<'r, AnimationIndexedAABB>>,
}


#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct AnimationName<'r>
{
    #[auto_struct(args)]
    info_type_count: u16,

    pub index: u32,
    #[auto_struct(init = bool_to_opt(info_type_count < 10))]
    pub unknown: Option<CStr<'r>>,
    pub name: CStr<'r>,
}


#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct PasDatabase<'r>
{
    #[auto_struct(expect = FourCC::from_bytes(b"PAS4"))]
    magic: FourCC,

    pub anim_state_count: u32,
    pub default_state: u32,
    #[auto_struct(init = (anim_state_count as usize, ()))]
    pub anim_states: RoArray<'r, PasAnimState<'r>>,
}

// PasDatabase inner details {{{

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct PasAnimState<'r>
{
    pub unknown: u32,
    pub param_info_count: u32,
    pub anim_info_count: u32,
    #[auto_struct(init = (param_info_count as usize, ()))]
    pub param_info: RoArray<'r, PasAnimStateParamInfo<'r>>,
    #[auto_struct(init = (anim_info_count as usize, param_info.clone()))]
    pub anim_info: RoArray<'r, PasAnimStateAnimInfo<'r>>,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct PasAnimStateParamInfo<'r>
{
    pub param_type: u32,
    pub unknown0: u32,
    pub unknown1: f32,
    #[auto_struct(init = (if param_type == 3 { 1 } else { 4 }, ()))]
    pub data0: RoArray<'r, u8>,
    #[auto_struct(init = (if param_type == 3 { 1 } else { 4 }, ()))]
    pub data1: RoArray<'r, u8>,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct PasAnimStateAnimInfo<'r>
{
    #[auto_struct(args)]
    param_info: RoArray<'r, PasAnimStateParamInfo<'r>>,

    pub unknown: u32,
    #[auto_struct(init = param_info.iter())]
    pub items: IteratorArray<'r, PasAnimStateAnimInfoInner<'r>, RoArrayIter<'r, PasAnimStateParamInfo<'r>>>,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct PasAnimStateAnimInfoInner<'r>
{
    #[auto_struct(args)]
    param_info: PasAnimStateParamInfo<'r>,
    #[auto_struct(init = (if param_info.param_type == 3 { 1 } else { 4 }, ()))]
    pub data0: RoArray<'r, u8>,
}

// }}}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct ParticleResData<'r>
{
    #[auto_struct(args)]
    info_type_count: u16,

    #[auto_struct(derive = part_assets.len() as u32)]
    pub part_asset_count: u32,
    #[auto_struct(init = (part_asset_count as usize, ()))]
    pub part_assets: LazyArray<'r, u32>,

    #[auto_struct(derive = swhc_assets.len() as u32)]
    pub swhc_asset_count: u32,
    #[auto_struct(init = (swhc_asset_count as usize, ()))]
    pub swhc_assets: RoArray<'r, ResId<SHWC>>,

    #[auto_struct(derive = unknowns.len() as u32)]
    pub unknown_count: u32,
    #[auto_struct(init = (unknown_count as usize, ()))]
    pub unknowns: RoArray<'r, u32>,

    #[auto_struct(init = bool_to_opt(info_type_count > 5))]
    pub elsc_count: Option<u32>,
    #[auto_struct(init = elsc_count.map(|i| (i as usize, ())))]
    pub elsc_assets: Option<RoArray<'r, ResId<ELSC>>>,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct AnimationAABB<'r>
{
    pub name: CStr<'r>,
    pub aabb: GenericArray<f32, U6>,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct AnimationIndexedAABB
{
    pub index: u32,
    pub aabb: GenericArray<f32, U6>,
}


#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Effect<'r>
{
    pub name: CStr<'r>,
    pub component_count: u32,
    #[auto_struct(init = (component_count as usize, ()))]
    pub components: RoArray<'r, EffectComponent<'r>>,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct EffectComponent<'r>
{
    pub name: CStr<'r>,
    pub type_: FourCC,
    pub file_id: u32,
    pub bone: CStr<'r>,
    pub scale: f32,
    pub parent_mode: u32,
    pub flags: u32,
}


#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct AnimationSet<'r>
{
    pub info_count: u16,

    #[auto_struct(derive = animations.len() as u32)]
    pub animation_count: u32,
    #[auto_struct(init = (animation_count as usize, ()))]
    pub animations: LazyArray<'r, Animation<'r>>,

    pub transition_count: u32,
    #[auto_struct(init = (transition_count as usize, ()))]
    pub transitions: RoArray<'r, Transition<'r>>,
    pub default_transition: MetaTransition<'r>,

    pub additive_animation_count: u32,
    #[auto_struct(init = (additive_animation_count as usize, ()))]
    pub additive_animations: RoArray<'r, AdditiveAnimation>,

    // Defalut AddaptiveAnimation data
    pub fade_in: f32,
    pub fade_out: f32,

    #[auto_struct(init = bool_to_opt(info_count > 2))]
    pub half_transition_count: Option<u32>,
    #[auto_struct(init = half_transition_count.map(|i| (i as usize, ())))]
    pub half_transitions: Option<RoArray<'r, HalfTransition<'r>>>,

    #[auto_struct(init = bool_to_opt(info_count > 3))]
    #[auto_struct(derive = animation_resources.as_ref().map(|a| a.len() as u32))]
    pub animation_resource_count: Option<u32>,
    #[auto_struct(init = animation_resource_count.map(|i| (i as usize, ())))]
    pub animation_resources: Option<LazyArray<'r, AnimationResource>>,
}


#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Animation<'r>
{
    pub name: CStr<'r>,
    pub meta: MetaAnimation<'r>,
}

// Uncached allows for recursion without the struct having infinite size
#[derive(Debug, Clone)]
pub enum MetaAnimation<'r>
{
    Play(Uncached<'r, MetaAnimationPlay<'r>>),
    Blend(Uncached<'r, MetaAnimationBlend<'r>>),
    PhaseBlend(Uncached<'r, MetaAnimationBlend<'r>>),
    Random(Uncached<'r, MetaAnimationRandom<'r>>),
    Sequence(Uncached<'r, MetaAnimationSequence<'r>>),
}


impl<'r> Readable<'r> for MetaAnimation<'r>
{
    type Args = ();
    fn read_from(reader: &mut Reader<'r>, (): ()) -> Self
    {
        let kind: u32 = reader.read(());
        let res = match kind {
            0 => MetaAnimation::Play(reader.read(())),
            1 => MetaAnimation::Blend(reader.read(())),
            2 => MetaAnimation::PhaseBlend(reader.read(())),
            3 => MetaAnimation::Random(reader.read(())),
            4 => MetaAnimation::Sequence(reader.read(())),
            n => panic!("Unexpected MetaAnimation tag: {}", n),
        };
        res
    }

    fn size(&self) -> usize
    {
        u32::fixed_size().unwrap() + match *self {
            MetaAnimation::Play(ref i) => i.size(),
            MetaAnimation::Blend(ref i) => i.size(),
            MetaAnimation::PhaseBlend(ref i) => i.size(),
            MetaAnimation::Random(ref i) => i.size(),
            MetaAnimation::Sequence(ref i) => i.size(),
        }
    }
}

impl<'r> Writable for MetaAnimation<'r>
{
    fn write_to<W: io::Write>(&self, writer: &mut W) -> io::Result<u64>
    {
        Ok(match self {
            MetaAnimation::Play(i) => 0u32.write_to(writer)? + i.write_to(writer)?,
            MetaAnimation::Blend(i) => 1u32.write_to(writer)? + i.write_to(writer)?,
            MetaAnimation::PhaseBlend(i) => 2u32.write_to(writer)? + i.write_to(writer)?,
            MetaAnimation::Random(i) => 3u32.write_to(writer)? + i.write_to(writer)?,
            MetaAnimation::Sequence(i) => 4u32.write_to(writer)? + i.write_to(writer)?,
        })
    }
}


#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct MetaAnimationPlay<'r>
{
    pub anim: ResId<ANIM>,
    pub index: u32,
    pub name: CStr<'r>,
    pub unknown0: f32,
    pub unknown1: u32,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct MetaAnimationBlend<'r>
{
    pub anim_a: MetaAnimation<'r>,
    pub anim_b: MetaAnimation<'r>,
    pub unknown0: f32,
    pub unknown1: u8,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct MetaAnimationRandom<'r>
{
    pub anim_count: u32,
    #[auto_struct(init = (anim_count as usize, ()))]
    pub anims: RoArray<'r, MetaAnimationRandomPair<'r>>,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct MetaAnimationRandomPair<'r>
{
    pub meta: MetaAnimation<'r>,
    pub probability: u32,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct MetaAnimationSequence<'r>
{
    pub anim_count: u32,
    #[auto_struct(init = (anim_count as usize, ()))]
    pub anims: RoArray<'r, MetaAnimation<'r>>,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Transition<'r>
{
    pub unknown: u32,
    pub anim_index_a: u32,
    pub anim_index_b: u32,
    pub meta: MetaTransition<'r>,
}

#[derive(Debug, Clone)]
pub enum MetaTransition<'r>
{
    Animation(Uncached<'r, MetaTransitionAnimation<'r>>),
    Transition(Uncached<'r, MetaTransitionTransition>),
    PhaseTransition(Uncached<'r, MetaTransitionTransition>),
    NoTransition,
}

impl<'r> Readable<'r> for MetaTransition<'r>
{
    type Args = ();
    fn read_from(reader: &mut Reader<'r>, (): ()) -> Self
    {
        let kind: u32 = reader.read(());
        let res = match kind {
            0 => MetaTransition::Animation(reader.read(())),
            1 => MetaTransition::Transition(reader.read(())),
            2 => MetaTransition::PhaseTransition(reader.read(())),
            3 => MetaTransition::NoTransition,
            _ => panic!("TODO"),
        };
        res
    }

    fn size(&self) -> usize
    {
        u32::fixed_size().unwrap() + match *self {
            MetaTransition::Animation(ref i) => i.size(),
            MetaTransition::Transition(ref i) => i.size(),
            MetaTransition::PhaseTransition(ref i) => i.size(),
            MetaTransition::NoTransition => 0,
        }
    }
}

impl<'r> Writable for MetaTransition<'r>
{
    fn write_to<W: io::Write>(&self, writer: &mut W) -> io::Result<u64>
    {
        Ok(match self {
            MetaTransition::Animation(i) => 0u32.write_to(writer)? + i.write_to(writer)?,
            MetaTransition::Transition(i) => 1u32.write_to(writer)? + i.write_to(writer)?,
            MetaTransition::PhaseTransition(i) => 2u32.write_to(writer)? + i.write_to(writer)?,
            MetaTransition::NoTransition => 3u32.write_to(writer)?,
        })
    }
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct MetaTransitionAnimation<'r>
{
    pub meta: MetaAnimation<'r>,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct MetaTransitionTransition
{
    pub time: f32,
    pub unknown0: u32,
    pub unknown1: u8,
    pub unknown2: u8,
    pub unknown3: u32,
}


#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct AdditiveAnimation
{
    pub index: u32,
    pub fade_in: f32,
    pub fade_out: f32,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct HalfTransition<'r>
{
    pub index: u32,
    pub meta: MetaTransition<'r>,
}

#[auto_struct(Readable, Writable, FixedSize)]
#[derive(Debug, Clone)]
pub struct AnimationResource
{
    pub anim: ResId<ANIM>,
    pub evnt: ResId<EVNT>,
}
