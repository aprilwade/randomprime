
use reader_writer::{CStr, FourCC, IteratorArray, Readable, Reader, RoArray, Uncached, RoArrayIter};
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;

fn bool_to_opt(b: bool) -> Option<()>
{
    if b {
        Some(())
    } else {
        None
    }
}

auto_struct! {
    #[auto_struct(Readable)]
    #[derive(Debug, Clone)]
    pub struct Ancs<'a>
    {
        #[expect = 1]
        version: u16,

        char_set: CharacterSet<'a>,
        anim_set: AnimationSet<'a>,
    }
}

auto_struct! {
    #[auto_struct(Readable)]
    #[derive(Debug, Clone)]
    pub struct CharacterSet<'a>
    {
        #[expect = 1]
        version: u16,

        char_info_count: u32,
        char_info: RoArray<'a, CharacterInfo<'a>> = (char_info_count as usize, ()),
    }
}

auto_struct! {
    #[auto_struct(Readable)]
    #[derive(Debug, Clone)]
    pub struct CharacterInfo<'a>
    {
        id: u32,

        info_type_count: u16,

        name: CStr<'a>,

        cmdl: u32,
        cskr: u32,
        cinf: u32,

        animation_count: u32,
        animation_names: RoArray<'a, AnimationName<'a>> = (animation_count as usize,
                                                           info_type_count),

        pas_database: PasDatabase<'a>,
        particles: ParticleResData<'a> = info_type_count,

        unknown0: u32,
        unknown1: Option<u32> = bool_to_opt(info_type_count > 9),
        unknown2: Option<u32> = bool_to_opt(info_type_count > 9),

        animation_aabb_count: Option<u32> = bool_to_opt(info_type_count > 1),
        animation_aabbs: Option<RoArray<'a, AnimationAABB<'a>>> =
            animation_aabb_count.map(|i| (i as usize, ())),

        effect_count: Option<u32> = bool_to_opt(info_type_count > 1),
        effects: Option<RoArray<'a, Effect<'a>>> = effect_count.map(|i| (i as usize, ())),

        overlay_cmdl: Option<u32> = bool_to_opt(info_type_count > 3),
        overlay_cskr: Option<u32> = bool_to_opt(info_type_count > 3),

        animation_index_count: Option<u32> = bool_to_opt(info_type_count > 4),
        animation_indices: Option<RoArray<'a, u32>> =
            animation_index_count.map(|i| (i as usize, ())),

        unknown3: Option<u32> = bool_to_opt(info_type_count > 9),
        unknown4: Option<u8> = bool_to_opt(info_type_count > 9),

        animation_indexed_aabb_count: Option<u32> = bool_to_opt(info_type_count > 9),
        animation_indexed_aabbs: Option<RoArray<'a, AnimationIndexedAABB>> =
            animation_indexed_aabb_count.map(|i| (i as usize, ())),
    }
}


auto_struct! {
    #[auto_struct(Readable)]
    #[derive(Debug, Clone)]
    pub struct AnimationName<'a>
    {
        #[args]
        info_type_count: u16,

        index: u32,
        unknown: Option<CStr<'a>> = bool_to_opt(info_type_count < 10),
        name: CStr<'a>,
    }
}


auto_struct! {
    #[auto_struct(Readable)]
    #[derive(Debug, Clone)]
    pub struct PasDatabase<'a>
    {
        #[expect = FourCC::from_bytes(b"PAS4")]
        magic: FourCC,

        anim_state_count: u32,
        default_state: u32,
        anim_states: RoArray<'a, PasAnimState<'a>> = (anim_state_count as usize, ()),
    }
}

// PasDatabase inner details {{{

auto_struct! {
    #[auto_struct(Readable)]
    #[derive(Debug, Clone)]
    pub struct PasAnimState<'a>
    {
        unknown: u32,
        param_info_count: u32,
        anim_info_count: u32,
        param_info: RoArray<'a, PasAnimStateParamInfo<'a>> = (param_info_count as usize, ()),
        anim_info: RoArray<'a, PasAnimStateAnimInfo<'a>> = (anim_info_count as usize,
                                                            param_info.clone())
    }
}

auto_struct! {
    #[auto_struct(Readable)]
    #[derive(Debug, Clone)]
    pub struct PasAnimStateParamInfo<'a>
    {
        param_type: u32,
        unknown0: u32,
        unknown1: f32,
        data0: RoArray<'a, u8> = (if param_type == 3 { 1 } else { 4 }, ()),
        data1: RoArray<'a, u8> = (if param_type == 3 { 1 } else { 4 }, ()),
    }
}

auto_struct! {
    #[auto_struct(Readable)]
    #[derive(Debug, Clone)]
    pub struct PasAnimStateAnimInfo<'a>
    {
        #[args]
        param_info: RoArray<'a, PasAnimStateParamInfo<'a>>,

        unknown: u32,
        items: IteratorArray<'a, PasAnimStateAnimInfoInner<'a>,
                                 RoArrayIter<'a, PasAnimStateParamInfo<'a>>> = param_info.iter(),
    }
}

auto_struct! {
    #[auto_struct(Readable)]
    #[derive(Debug, Clone)]
    pub struct PasAnimStateAnimInfoInner<'a>
    {
        #[args]
        param_info: PasAnimStateParamInfo<'a>,
        data0: RoArray<'a, u8> = (if param_info.param_type == 3 { 1 } else { 4 }, ()),
    }
}

// }}}

auto_struct! {
    #[auto_struct(Readable)]
    #[derive(Debug, Clone)]
    pub struct ParticleResData<'a>
    {
        #[args]
        info_type_count: u16,

        part_asset_count: u32,
        part_assets: RoArray<'a, u32> = (part_asset_count as usize, ()),

        swhc_asset_count: u32,
        swhc_assets: RoArray<'a, u32> = (swhc_asset_count as usize, ()),

        unknown_count: u32,
        unknowns: RoArray<'a, u32> = (unknown_count as usize, ()),

        elsc_count: Option<u32> = bool_to_opt(info_type_count > 5),
        elsc_assets: Option<RoArray<'a, u32>> = elsc_count.map(|i| (i as usize, ())),
    }
}

auto_struct! {
    #[auto_struct(Readable)]
    #[derive(Debug, Clone)]
    pub struct AnimationAABB<'a>
    {
        name: CStr<'a>,
        aabb: GenericArray<f32, U6>,
    }
}

auto_struct! {
    #[auto_struct(Readable)]
    #[derive(Debug, Clone)]
    pub struct AnimationIndexedAABB
    {
        index: u32,
        aabb: GenericArray<f32, U6>,
    }
}


auto_struct! {
    #[auto_struct(Readable)]
    #[derive(Debug, Clone)]
    pub struct Effect<'a>
    {
        name: CStr<'a>,
        component_count: u32,
        components: RoArray<'a, EffectComponent<'a>> = (component_count as usize, ()),
    }
}

auto_struct! {
    #[auto_struct(Readable)]
    #[derive(Debug, Clone)]
    pub struct EffectComponent<'a>
    {
        name: CStr<'a>,
        type_: FourCC,
        file_id: u32,
        bone: CStr<'a>,
        scale: f32,
        parent_mode: u32,
        flags: u32,
    }
}


auto_struct! {
    #[auto_struct(Readable)]
    #[derive(Debug, Clone)]
    pub struct AnimationSet<'a>
    {
        info_count: u16,
        animation_count: u32,
        animations: RoArray<'a, Animation<'a>> = (animation_count as usize, ()),

        transition_count: u32,
        transitions: RoArray<'a, Transition<'a>> = (transition_count as usize, ()),
        default_transition: MetaTransition<'a>,

        additive_animation_count: u32,
        additive_animations: RoArray<'a, AdditiveAnimation> =
            (additive_animation_count as usize, ()),

        // Defalut AddaptiveAnimation data
        fade_in: f32,
        fade_out: f32,

        half_transition_count: Option<u32> = bool_to_opt(info_count > 2),
        half_transitions: Option<RoArray<'a, HalfTransition<'a>>> =
            half_transition_count.map(|i| (i as usize, ())),

        animation_resource_count: Option<u32> = bool_to_opt(info_count > 3),
        animation_resources: Option<RoArray<'a, AnimationResource>> =
            animation_resource_count.map(|i| (i as usize, ())),
    }
}


auto_struct! {
    #[auto_struct(Readable)]
    #[derive(Debug, Clone)]
    pub struct Animation<'a>
    {
        name: CStr<'a>,
        meta: MetaAnimation<'a>,
    }
}

// Uncached allows for recursion without the struct having infinite size
#[derive(Debug, Clone)]
pub enum MetaAnimation<'a>
{
    Play(Uncached<'a, MetaAnimationPlay<'a>>),
    Blend(Uncached<'a, MetaAnimationBlend<'a>>),
    PhaseBlend(Uncached<'a, MetaAnimationBlend<'a>>),
    Random(Uncached<'a, MetaAnimationRandom<'a>>),
    Sequence(Uncached<'a, MetaAnimationSequence<'a>>),
}


impl<'a> Readable<'a> for MetaAnimation<'a>
{
    type Args = ();
    fn read(mut reader: Reader<'a>, (): ()) -> (Self, Reader<'a>)
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
        (res, reader)
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


auto_struct! {
    #[auto_struct(Readable)]
    #[derive(Debug, Clone)]
    pub struct MetaAnimationPlay<'a>
    {
        anim: u32,
        index: u32,
        name: CStr<'a>,
        unknown0: f32,
        unknown1: u32,
    }
}

auto_struct! {
    #[auto_struct(Readable)]
    #[derive(Debug, Clone)]
    pub struct MetaAnimationBlend<'a>
    {
        anim_a: MetaAnimation<'a>,
        anim_b: MetaAnimation<'a>,
        unknown0: f32,
        unknown1: u8,
    }
}

auto_struct! {
    #[auto_struct(Readable)]
    #[derive(Debug, Clone)]
    pub struct MetaAnimationRandom<'a>
    {
        anim_count: u32,
        anims: RoArray<'a, MetaAnimationRandomPair<'a>> = (anim_count as usize, ()),
    }
}

auto_struct! {
    #[auto_struct(Readable)]
    #[derive(Debug, Clone)]
    pub struct MetaAnimationRandomPair<'a>
    {
        meta: MetaAnimation<'a>,
        probability: u32,
    }
}

auto_struct! {
    #[auto_struct(Readable)]
    #[derive(Debug, Clone)]
    pub struct MetaAnimationSequence<'a>
    {
        anim_count: u32,
        anims: RoArray<'a, MetaAnimation<'a>> = (anim_count as usize, ()),
    }
}

auto_struct! {
    #[auto_struct(Readable)]
    #[derive(Debug, Clone)]
    pub struct Transition<'a>
    {
        unknown: u32,
        anim_index_a: u32,
        anim_index_b: u32,
        meta: MetaTransition<'a>,
    }
}

#[derive(Debug, Clone)]
pub enum MetaTransition<'a>
{
    Animation(Uncached<'a, MetaTransitionAnimation<'a>>),
    Transition(Uncached<'a, MetaTransitionTransition>),
    PhaseTransition(Uncached<'a, MetaTransitionTransition>),
    NoTransition,
}

impl<'a> Readable<'a> for MetaTransition<'a>
{
    type Args = ();
    fn read(mut reader: Reader<'a>, (): ()) -> (Self, Reader<'a>)
    {
        let kind: u32 = reader.read(());
        let res = match kind {
            0 => MetaTransition::Animation(reader.read(())),
            1 => MetaTransition::Transition(reader.read(())),
            2 => MetaTransition::PhaseTransition(reader.read(())),
            3 => MetaTransition::NoTransition,
            _ => panic!("TODO"),
        };
        (res, reader)
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

auto_struct! {
    #[auto_struct(Readable)]
    #[derive(Debug, Clone)]
    pub struct MetaTransitionAnimation<'a>
    {
        meta: MetaAnimation<'a>,
    }
}

auto_struct! {
    #[auto_struct(Readable, FixedSize)]
    #[derive(Debug, Clone)]
    pub struct MetaTransitionTransition
    {
        time: f32,
        unknown0: u32,
        unknown1: u8,
        unknown2: u8,
        unknown3: u32,
    }
}


auto_struct! {
    #[auto_struct(Readable, FixedSize)]
    #[derive(Debug, Clone)]
    pub struct AdditiveAnimation
    {
        index: u32,
        fade_in: f32,
        fade_out: f32,
    }
}

auto_struct! {
    #[auto_struct(Readable)]
    #[derive(Debug, Clone)]
    pub struct HalfTransition<'a>
    {
        index: u32,
        meta: MetaTransition<'a>,
    }
}

auto_struct! {
    #[auto_struct(Readable, FixedSize)]
    #[derive(Debug, Clone)]
    pub struct AnimationResource
    {
        anim: u32,
        evnt: u32,
    }
}
