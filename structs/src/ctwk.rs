use auto_struct_macros::auto_struct;
use reader_writer::{
    Reader, Readable, Writable, CStr, generic_array::GenericArray, typenum::*,
};
use std::io;

#[derive(Clone, Debug)]
pub enum Ctwk<'r>
{
    CtwkGame(CtwkGame<'r>),
    CtwkPlayer(CtwkPlayer<'r>),
    CtwkPlayerGun(CtwkPlayerGun<'r>),
    CtwkBall(CtwkBall<'r>),
    CtwkGuiColors(CtwkGuiColors<'r>),
}

impl<'r> Writable for Ctwk<'r>
{
    fn write_to<W: io::Write>(&self, writer: &mut W) -> io::Result<u64>
    {
        match self {
            Ctwk::CtwkGame(ctwk) => ctwk.write_to(writer),
            Ctwk::CtwkPlayer(ctwk) => ctwk.write_to(writer),
            Ctwk::CtwkPlayerGun(ctwk) => ctwk.write_to(writer),
            Ctwk::CtwkBall(ctwk) => ctwk.write_to(writer),
            Ctwk::CtwkGuiColors(ctwk) => ctwk.write_to(writer),
        }
    }
}

impl<'r> Readable<'r> for Ctwk<'r>
{
    type Args = ();
    fn read_from(reader: &mut Reader<'r>, (): ()) -> Self
    {
        // TODO: This will not work for every CTWK, need a way to differentiate:
        //  - PlayerControls from PlayerControls2 (size == 288)
        //  - Ball from GunRes (size == 480)
        match reader.len() {
             96 => Ctwk::CtwkGame(reader.read(())),
            800 => Ctwk::CtwkPlayer(reader.read(())),
            512 => Ctwk::CtwkPlayerGun(reader.read(())),
            480 => Ctwk::CtwkBall(reader.read(())),
            2368 => Ctwk::CtwkGuiColors(reader.read(())),
            _ => panic!("Unhandled CTWK size - {}", reader.len()),
        }
    }

    fn size(&self) -> usize
    {
        match self {
            Ctwk::CtwkGame(ctwk) => ctwk.size(),
            Ctwk::CtwkPlayer(ctwk) => ctwk.size(),
            Ctwk::CtwkPlayerGun(ctwk) => ctwk.size(),
            Ctwk::CtwkBall(ctwk) => ctwk.size(),
            Ctwk::CtwkGuiColors(ctwk) => ctwk.size(),
        }
    }
}

#[auto_struct(Readable, Writable)]
#[derive(Clone, Debug)]
pub struct CtwkGame<'r>
{
    pub start: Reader<'r>,
    pub world_prefix: CStr<'r>,
    pub default_room: CStr<'r>,
    pub fov: f32,
    pub unknown1: u8,
    pub unknown2: u8,
    pub unknown3: u8,
    pub splash_screens_disabled: u8,
    pub unknown5: f32,
    pub press_start_delay: f32,
    pub wavecap_intensity_normal: f32,
    pub wavecap_intensity_poison: f32,
    pub wavecap_intensity_lava: f32,
    pub ripple_intensity_normal: f32,
    pub ripple_intensity_poison: f32,
    pub ripple_intensity_lava: f32,
    pub fluid_env_bump_scale: f32,
    pub water_fog_distance_base: f32,
    pub water_fog_distance_range: f32,
    pub gravity_water_fog_distance_base: f32,
    pub gravity_water_fog_distance_range: f32,
    pub hardmode_damage_mult: f32,
    pub hardmode_weapon_mult: f32,

    #[auto_struct(pad_align = 32)]
    _pad: (),
}

#[auto_struct(Readable, Writable)]
#[derive(Clone, Debug)]
pub struct CtwkPlayer<'r>
{
    // Copied from URDE. Note that the URDE header files are arbitrarily ordered. You need to view the .cpp to see the actual order
    pub start: Reader<'r>,
    pub max_translational_acceleration: GenericArray<f32, U8>,
    pub max_rotational_acceleration: GenericArray<f32, U8>,
    pub translation_friction: GenericArray<f32, U8>,
    pub rotation_friction: GenericArray<f32, U8>,
    pub rotation_max_speed: GenericArray<f32, U8>,
    pub translation_max_speed: GenericArray<f32, U8>,
    pub normal_grav_accel: f32,
    pub fluid_grav_accel: f32,
    pub vertical_jump_accel: f32,
    pub horizontal_jump_accel: f32,
    pub vertical_double_jump_accel: f32,
    pub horizontal_double_jump_accel: f32,
    pub water_jump_factor: f32,
    pub water_ball_jump_factor: f32,
    pub lava_jump_factor: f32,
    pub lava_ball_jump_factor: f32,
    pub phazon_jump_factor: f32,
    pub phazon_ball_jump_factor: f32,
    pub allowed_jump_time: f32,
    pub allowed_double_jump_time: f32,
    pub min_double_jump_window: f32,
    pub max_double_jump_window: f32,
    pub unknown0: f32,
    pub min_jump_time: f32,
    pub min_double_jump_time: f32,
    pub allowed_ledge_time: f32,
    pub double_jump_impulse: f32,
    pub backwards_force_multiplier: f32,
    pub bomb_jump_radius: f32,
    pub bomb_jump_height: f32,
    pub eye_offset: f32,
    pub turn_speed_multiplier: f32,
    pub free_look_turn_speed_multiplier: f32,
    pub horizontal_free_look_angle_vel: f32,
    pub vertical_free_look_angle_vel: f32,
    pub free_look_speed: f32,
    pub free_look_snap_speed: f32,
    pub unknown1: f32,
    pub free_look_centered_threshold_angle: f32,
    pub free_look_centered_time: f32,
    pub free_look_dampen_factor: f32,
    pub left_div: f32,
    pub right_div: f32,
    pub freelook_turns_player: u8,
    pub unknownbool_25: u8,
    pub unknownbool_26: u8,
    pub move_during_free_look: u8,
    pub hold_buttons_for_free_look: u8,
    pub two_buttons_for_free_look: u8,
    pub unknownbool_30: u8,
    pub unknownbool_31: u8,
    pub unknownbool_24: u8,
    pub aim_when_orbiting_point: u8,
    pub stay_in_free_look_while_firing: u8,
    pub unknownbool_27: u8,
    pub unknownbool_28: u8,
    pub orbit_fixed_offset: u8,
    pub gun_button_toggles_holster: u8,
    pub gun_not_firing_holsters_gun: u8,
    pub falling_double_jump: u8,
    pub impulse_double_jump: u8,
    pub firing_cancels_camera_pitch: u8,
    pub assisted_aiming_ignore_horizontal: u8,
    pub assisted_aiming_ignore_vertical: u8,
    pub unknown10: f32,
    pub unknown11: f32,
    pub aim_max_distance: f32,
    pub unknown12: f32,
    pub unknown13: f32,
    pub unknown15: f32,
    pub unknown16: f32,
    pub unknown17: f32,
    pub aim_threshold_distance: f32,
    pub unknown18: f32,
    pub unknown19: f32,
    pub aim_box_width: f32,
    pub aim_box_height: f32,
    pub aim_target_timer: f32,
    pub aim_assist_horizontal_angle: f32,
    pub aim_assist_vertical_angle: f32,
    pub orbit_min_distance: GenericArray<f32, U3>,
    pub orbit_normal_distance: GenericArray<f32, U3>,
    pub orbit_max_distance: GenericArray<f32, U3>,
    pub unknown2: f32,
    pub orbit_mode_timer: f32,
    pub orbit_camera_speed: f32,
    pub orbit_upper_angle: f32,
    pub orbit_lower_angle: f32,
    pub orbit_horiz_angle: f32,
    pub unknown3: f32,
    pub unknown4: f32,
    pub orbit_max_target_distance: f32,
    pub orbit_max_lock_distance: f32,
    pub orbit_distance_threshold: f32,
    pub orbit_screen_box_half_extent_x: GenericArray<u32, U2>,
    pub orbit_screen_box_half_extent_y: GenericArray<u32, U2>,
    pub orbit_screen_box_center_x: GenericArray<u32, U2>,
    pub orbit_screen_box_center_y: GenericArray<u32, U2>,
    pub orbit_zone_ideal_x: GenericArray<u32, U2>,
    pub orbit_zone_ideal_y: GenericArray<u32, U2>,
    pub orbit_near_x: f32,
    pub orbit_near_z: f32,
    pub unknown5: f32,
    pub unknown6: f32,
    pub orbit_fixed_offset_z_diff: f32,
    pub orbit_z_range: f32,
    pub unknown7: f32,
    pub unknown8: f32,
    pub unknown9: f32,
    pub orbit_prevention_time: f32,
    pub dash_enabled: u8,
    pub dash_on_button_release: u8,
    pub dash_button_hold_cancel_time: f32,
    pub dash_strafe_input_threshold: f32,
    pub sideways_double_jump_impulse: f32,
    pub sideways_vertical_double_jump_accel: f32,
    pub sideways_horizontal_double_jump_accel: f32,
    pub scanning_range: f32,
    pub scan_retention: u8,
    pub scan_freezes_game: u8,
    pub orbit_while_scanning: u8,
    pub scan_max_target_distance: f32,
    pub scan_max_lock_distance: f32,
    pub orbit_distance_max: f32,
    pub grapple_swing_length: f32,
    pub grapple_swing_period: f32,
    pub grapple_pull_speed_min: f32,
    pub grapple_camera_speed: f32,
    pub max_grapple_locked_turn_align_distance: f32,
    pub grapple_pull_speed_proportion: f32,
    pub grapple_pull_speed_max: f32,
    pub grapple_look_center_speed: f32,
    pub max_grapple_turn_speed: f32,
    pub grapple_jump_force: f32,
    pub grapple_release_time: f32,
    pub grapple_jump_mode: u32,
    pub orbit_release_breaks_grapple: u8,
    pub invert_grapple_turn: u8,
    pub grapple_beam_speed: f32,
    pub grapple_beam_x_wave_amplitude: f32,
    pub grapple_beam_z_wave_amplitude: f32,
    pub grapple_beam_angle_phase_delta: f32,
    pub player_height: f32,
    pub player_xy_half_extent: f32,
    pub step_up_height: f32,
    pub step_down_height: f32,
    pub player_ball_half_extent: f32,
    pub first_person_camera_speed: f32,
    pub unknown20: f32,
    pub jump_camera_pitch_down_start: f32,
    pub jump_camera_pitch_down_full: f32,
    pub jump_camera_pitch_down_angle: f32,
    pub fall_camera_pitch_down_start: f32,
    pub fall_camera_pitch_down_full: f32,
    pub fall_camera_pitch_down_angle: f32,
    pub unknown21: f32,
    pub unknown22: f32,
    pub unknown23: f32,
    pub unknown24: u8,
    pub frozen_timeout: f32,
    pub ice_break_jump_count: u32,
    pub varia_damage_reduction: f32,
    pub gravity_damage_reduction: f32,
    pub phazon_damage_reduction: f32,

    #[auto_struct(pad_align = 32)]
    _pad: (),
}

#[auto_struct(Readable, Writable)]
#[derive(Clone, Debug)]
pub struct SShotParam
{
    pub weapon_type: i32,
//    pub charged : u8,
//    pub combo : u8,
//    pub insta_kill : u8,
    pub damage: f32,
    pub radius_damage: f32,
    pub radius: f32,
    pub knockback: f32,
//    pub no_immunity: u8,
}

#[auto_struct(Readable, Writable)]
#[derive(Clone, Debug)]
pub struct SWeaponInfo
{
    pub cool_down: f32,
    pub normal: SShotParam,
    pub charged: SShotParam,
}

#[auto_struct(Readable, Writable)]
#[derive(Clone, Debug)]
pub struct CtwkPlayerGun<'r>
{
    pub start: Reader<'r>,
    pub up_look_angle: f32,
    pub down_look_angle: f32,
    pub vertical_spread: f32,
    pub horizontal_spread: f32,
    pub high_vertical_spread: f32,
    pub high_horizontal_spread: f32,
    pub low_vertical_spread: f32,
    pub low_horizontal_spread: f32,
    pub aim_vertical_speed: f32,
    pub aim_horizontal_speed: f32,
    pub bomb_fuse_time: f32,
    pub bomb_drop_delay_time: f32,
    pub holo_hold_time: f32,
    pub gun_transform_time: f32,
    pub gun_holster_time: f32,
    pub gun_not_firing_time: f32,
    pub fixed_vertical_aim: f32,
    pub gun_extend_distance: f32,
    pub gun_position: GenericArray<f32, U3>,
    pub unknown0: GenericArray<f32, U3>,
    pub grappling_arm_position: GenericArray<f32, U3>,
    pub bomb: SShotParam,
    pub power_bomb: SShotParam,
    pub missile: SShotParam,
    pub beams: GenericArray<SWeaponInfo, U5>,
    pub combos: GenericArray<SShotParam, U5>,
    pub ricochet_data: GenericArray<f32, U6>,

    #[auto_struct(pad_align = 32)]
    _pad: (),
}

#[auto_struct(Readable, Writable)]
#[derive(Clone, Debug)]
pub struct CtwkBall<'r>
{
    pub start: Reader<'r>,
    pub max_translation_accel: GenericArray<f32, U8>,
    pub translation_friction: GenericArray<f32, U8>,
    pub translation_max_speed: GenericArray<f32, U8>,
    pub unknown0: GenericArray<f32, U4>,
    pub ball_forward_braking_accel: GenericArray<f32, U8>,
    pub ball_gravity: f32,
    pub ball_water_gravity: f32,
    pub unknown1: GenericArray<f32, U3>,
    pub dont_care0: GenericArray<f32, U27>,
    pub unknown2: GenericArray<f32, U6>,
    pub conservative_door_cam_distance: f32,
    pub unknown3: f32,
    pub dont_care1: GenericArray<f32, U27>,
    pub boost_drain_time: f32,
    pub boost_min_charge_time: f32,
    pub boost_min_rel_speed_for_damage: f32,
    pub boost_charge_time0: f32,
    pub boost_charge_time1: f32,
    pub boost_charge_time2: f32,
    pub boost_incremental_speed0: f32,
    pub boost_incremental_speed1: f32,
    pub boost_incremental_speed2: f32,
    pub filler: GenericArray<u8, U32>,

    #[auto_struct(pad_align = 32)]
    _pad: (),
}

#[auto_struct(Readable, Writable)]
#[derive(Clone, Debug)]
pub struct CtwkGuiColors<'r>
{
    pub start: Reader<'r>,
    pub colors: GenericArray<GenericArray<f32,U4>, U112>, // Set of 112 RGBA values
    pub visor_count: u32,
    pub visor_colors: GenericArray<GenericArray<GenericArray<f32,U4>, U7>, U5>, // Set of 7 RGBA values repeated for 5 visors

    #[auto_struct(pad_align = 32)]
    _pad: (),
}
