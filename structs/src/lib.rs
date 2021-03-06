
pub mod res_id;

mod ancs;
mod anim;
mod bnr;
mod cmdl;
mod dol;
mod evnt;
mod font;
mod frme;
mod gc_disc;
mod hint;
mod mlvl;
mod mrea;
mod pak;
mod part;
mod savw;
mod scan;
mod scly;
mod strg;
mod thp;
mod txtr;


mod scly_props
{
    // http://www.metroid2002.com/retromodding/wiki/User:Parax0/Sandbox
    pub mod actor;
    pub mod damageable_trigger;
    pub mod dock;
    pub mod door;
    pub mod effect;
    pub mod hud_memo;
    pub mod memory_relay;
    pub mod pickup;
    pub mod platorm;
    pub mod point_of_interest;
    pub mod player_actor;
    pub mod player_hint;
    pub mod relay;
    pub mod sound;
    pub mod spawn_point;
    pub mod special_function;
    pub mod streamed_audio;
    pub mod timer;
    pub mod trigger;
    pub mod world_transporter;

    pub mod structs;

    pub use self::actor::*;
    pub use self::damageable_trigger::*;
    pub use self::dock::*;
    pub use self::door::*;
    pub use self::effect::*;
    pub use self::hud_memo::*;
    pub use self::memory_relay::*;
    pub use self::pickup::*;
    pub use self::platorm::*;
    pub use self::point_of_interest::*;
    pub use self::player_actor::*;
    pub use self::player_hint::*;
    pub use self::relay::*;
    pub use self::sound::*;
    pub use self::spawn_point::*;
    pub use self::special_function::*;
    pub use self::streamed_audio::*;
    pub use self::timer::*;
    pub use self::trigger::*;
    pub use self::world_transporter::*;
}
pub use scly_props::structs as scly_structs;
pub use scly_props::actor::*;
pub use scly_props::damageable_trigger::*;
pub use scly_props::dock::*;
pub use scly_props::door::*;
pub use scly_props::effect::*;
pub use scly_props::hud_memo::*;
pub use scly_props::memory_relay::*;
pub use scly_props::pickup::*;
pub use scly_props::platorm::*;
pub use scly_props::point_of_interest::*;
pub use scly_props::player_actor::*;
pub use scly_props::player_hint::*;
pub use scly_props::relay::*;
pub use scly_props::sound::*;
pub use scly_props::spawn_point::*;
pub use scly_props::special_function::*;
pub use scly_props::streamed_audio::*;
pub use scly_props::timer::*;
pub use scly_props::trigger::*;
pub use scly_props::world_transporter::*;

pub use res_id::ResId;

pub use anim::*;
pub use ancs::*;
pub use bnr::*;
pub use cmdl::*;
pub use dol::*;
pub use evnt::*;
pub use font::*;
pub use frme::*;
pub use gc_disc::*;
pub use hint::*;
pub use mlvl::*;
pub use mrea::*;
pub use pak::*;
pub use part::*;
pub use savw::*;
pub use scan::*;
pub use scly::*;
pub use strg::*;
pub use thp::*;
pub use txtr::*;
