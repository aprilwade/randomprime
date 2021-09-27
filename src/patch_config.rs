use std::{
    ffi::CStr,
    collections::HashMap,
    fmt,
    fs::{File, OpenOptions},
    fs,
};

use clap::{
    Arg,
    App,
    crate_version,
};

use serde::Deserialize;

use crate::{
    starting_items::StartingItems,
    pickup_meta::PickupType,
};

/*** Parsed Config (fn patch_iso) ***/

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum IsoFormat
{
    Iso,
    Gcz,
    Ciso,
}

#[derive(Deserialize, Debug, Copy, Clone)]
#[serde(rename_all = "camelCase")]
pub enum ArtifactHintBehavior
{
    Default,
    None,
    All,
}

#[derive(PartialEq, Debug, Deserialize, Copy, Clone)]
#[serde(rename_all = "camelCase")]
pub enum MapState
{
    Default,
    Visible,
    Visited,
}

impl fmt::Display for MapState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(PartialEq, Debug, Deserialize, Copy, Clone)]
#[serde(rename_all = "camelCase")]
pub enum CutsceneMode
{
    Original,
    Competitive,
    Minor,
    Major,
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct GameBanner
{
    pub game_name: Option<String>,
    pub game_name_full: Option<String>,
    pub developer: Option<String>,
    pub developer_full: Option<String>,
    pub description: Option<String>,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PickupConfig
{
    #[serde(alias  = "type")]
    pub pickup_type: String,
    pub curr_increase: Option<i32>,
    pub max_increase: Option<i32>,
    pub model: Option<String>,
    pub scan_text: Option<String>,
    pub hudmemo_text: Option<String>,
    pub respawn: Option<bool>,
    pub position: Option<[f32;3]>,
    pub modal_hudmemo: Option<bool>,
    // pub desination: Option<String>,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScanConfig
{
    pub position: [f32;3],
    pub text: String,
    pub is_red: bool,
}

// TODO: defaults
#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RoomConfig
{
    // pub remove_locks: Option<bool>,
    // pub superheated: Option<i8>,
    // pub remove_water: Option<bool>,
    // pub submerge: Option<bool>,
    // pub extra_water: Option<Vec<WaterConfig>>,
    // pub doors: Option<Vec<String>>,
    // pub blast_shields: Option<Vec<String>>,
    pub pickups: Option<Vec<PickupConfig>>,
    // pub extra_pickups: Option<Vec<PickupConfig>>,
    pub extra_scans: Option<Vec<ScanConfig>>,
    // pub aether_transform: Option<Vec<AetherTransformConfig>>,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LevelConfig
{
    pub transports: HashMap<String, String>,
    pub rooms: HashMap<String, RoomConfig>,
}


#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CtwkConfig
{
    pub fov: Option<f32>,
    pub player_size: Option<f32>,
    pub morph_ball_size: Option<f32>,
    pub easy_lava_escape: Option<bool>,
    pub move_while_scan: Option<bool>,
    pub scan_range: Option<f32>,
    pub bomb_jump_height: Option<f32>,
    pub bomb_jump_radius: Option<f32>,
    pub grapple_beam_speed: Option<f32>,
    pub aim_assist_angle: Option<f32>,
    pub gravity: Option<f32>,
    pub ice_break_timeout: Option<f32>,
    pub ice_break_jump_count: Option<u32>,
    pub ground_friction: Option<f32>,
    pub coyote_frames: Option<u32>,
    pub move_during_free_look: Option<bool>,
    pub recenter_after_freelook: Option<bool>,
    pub max_speed: Option<f32>,
    pub max_acceleration: Option<f32>,
    pub space_jump_impulse: Option<f32>,
    pub vertical_space_jump_accel: Option<f32>,
    pub horizontal_space_jump_accel: Option<f32>,
    pub eye_offset: Option<f32>,
    pub toggle_free_look: Option<bool>,
    pub two_buttons_for_free_look: Option<bool>,
    pub disable_dash: Option<bool>,
    pub varia_damage_reduction: Option<f32>,
    pub gravity_damage_reduction: Option<f32>,
    pub phazon_damage_reduction: Option<f32>,
    pub hardmode_damage_mult: Option<f32>,
    pub hardmode_weapon_mult: Option<f32>,
    pub turn_speed: Option<f32>,
    pub underwater_fog_distance: Option<f32>,
    pub gun_position: Option<[f32;3]>,
    pub step_up_height: Option<f32>,
    pub allowed_jump_time: Option<f32>,
    pub allowed_space_jump_time: Option<f32>,
    pub min_space_jump_window: Option<f32>,
    pub max_space_jump_window: Option<f32>,
    pub min_jump_time: Option<f32>,
    pub min_space_jump_time: Option<f32>,
    pub falling_space_jump: Option<bool>,
    pub impulse_space_jump: Option<bool>,

    // Ball.CTWK
    pub max_translation_accel: Option<f32>,
    pub translation_friction: Option<f32>,
    pub translation_max_speed: Option<f32>,
    pub ball_forward_braking_accel: Option<f32>,
    pub ball_gravity: Option<f32>,
    pub ball_water_gravity: Option<f32>,
    pub boost_drain_time: Option<f32>,
    pub boost_min_charge_time: Option<f32>,
    pub boost_min_rel_speed_for_damage: Option<f32>,
    pub boost_charge_time0: Option<f32>,
    pub boost_charge_time1: Option<f32>,
    pub boost_charge_time2: Option<f32>,
    pub boost_incremental_speed0: Option<f32>,
    pub boost_incremental_speed1: Option<f32>,
    pub boost_incremental_speed2: Option<f32>,
}

#[derive(Debug)]
pub struct PatchConfig
{
    pub extern_assets_dir: Option<String>,
    pub seed: u64,

    pub force_vanilla_layout: bool,

    pub input_iso: memmap::Mmap,
    pub iso_format: IsoFormat,
    pub output_iso: File,

    pub qol_cutscenes: CutsceneMode,
    pub qol_game_breaking: bool,
    pub qol_cosmetic: bool,
    pub qol_pickup_scans: bool,

    pub phazon_elite_without_dynamo: bool,
    pub main_plaza_door: bool,
    pub backwards_labs: bool,
    pub backwards_frigate: bool,
    pub backwards_upper_mines: bool,
    pub backwards_lower_mines: bool,

    pub level_data: HashMap<String, LevelConfig>,

    pub starting_room: String,
    pub starting_memo: Option<String>,
    pub warp_to_start: bool,

    pub automatic_crash_screen: bool,
    pub etank_capacity: u32,
    pub nonvaria_heat_damage: bool,
    pub heat_damage_per_sec: f32,
    pub staggered_suit_damage: bool,
    pub item_max_capacity: HashMap<PickupType, u32>,
    pub map_default_state: MapState,
    pub auto_enabled_elevators: bool,
    pub multiworld_dol_patches: bool,
    pub update_hint_state_replacement: Option<Vec<u8>>,
    pub quiet: bool,

    pub starting_items: StartingItems,
    pub item_loss_items: StartingItems,

    pub artifact_hint_behavior: ArtifactHintBehavior,

    pub flaahgra_music_files: Option<[nod_wrapper::FileWrapper; 2]>,

    pub suit_hue_rotate_angle: Option<i32>,

    pub quickplay: bool,

    pub game_banner: GameBanner,
    pub comment: String,
    pub main_menu_message: String,

    pub credits_string: Option<String>,
    pub artifact_hints: Option<HashMap<String,String>>, // e.g. "Strength":"This item can be found in Ruined Fountain"
    pub artifact_temple_layer_overrides: Option<HashMap<String,bool>>,
    pub ctwk_config: CtwkConfig,
}

/*** Un-Parsed Config (doubles as JSON input specification) ***/

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
struct Preferences
{
    qol_game_breaking: Option<bool>,
    qol_cosmetic: Option<bool>,
    qol_logical: Option<bool>,
    qol_cutscenes: Option<String>,
    qol_pickup_scans: Option<bool>,

    map_default_state: Option<String>,
    artifact_hint_behavior: Option<String>,
    automatic_crash_screen: Option<bool>,
    trilogy_disc_path: Option<String>,
    quickplay: Option<bool>,
    quiet: Option<bool>,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
struct GameConfig
{
    starting_room: Option<String>,
    starting_memo: Option<String>,
    warp_to_start: Option<bool>,

    nonvaria_heat_damage: Option<bool>,
    staggered_suit_damage: Option<bool>,
    heat_damage_per_sec: Option<f32>,
    auto_enabled_elevators: Option<bool>,
    multiworld_dol_patches: Option<bool>,
    update_hint_state_replacement: Option<Vec<u8>>,

    starting_items: Option<StartingItems>,
    item_loss_items: Option<StartingItems>,

    etank_capacity: Option<u32>,
    item_max_capacity: Option<HashMap<String,u32>>,

    phazon_elite_without_dynamo: Option<bool>,
    main_plaza_door: Option<bool>,
    backwards_labs: Option<bool>,
    backwards_frigate: Option<bool>,
    backwards_upper_mines: Option<bool>,
    backwards_lower_mines: Option<bool>,

    game_banner: Option<GameBanner>,
    comment: Option<String>,
    main_menu_message: Option<String>,

    credits_string: Option<String>,
    artifact_hints: Option<HashMap<String,String>>, // e.g. "Strength":"This item can be found in Ruined Fountain"
    artifact_temple_layer_overrides: Option<HashMap<String,bool>>,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
struct PatchConfigPrivate
{
    input_iso: Option<String>,
    output_iso: Option<String>,
    force_vanilla_layout: Option<bool>,
    extern_assets_dir: Option<String>,
    seed: Option<u64>,

    #[serde(default)]
    preferences: Preferences,

    #[serde(default)]
    game_config: GameConfig,

    #[serde(default)]
    tweaks: CtwkConfig,

    #[serde(default)]
    level_data: HashMap<String, LevelConfig>,
}

/*** Parse Patcher Input ***/

impl PatchConfig
{
    pub fn from_json(json: &str) -> Result<Self, String>
    {
        let json_config: PatchConfigPrivate = serde_json::from_str(json)
            .map_err(|e| format!("JSON parse failed: {}", e))?;
        json_config.parse()
    }

    pub fn from_cli_options() -> Result<Self, String>
    {
        let matches = App::new("randomprime ISO patcher")
            .version(crate_version!())
            .arg(Arg::with_name("input iso path")
                .long("input-iso")
                .takes_value(true))
            .arg(Arg::with_name("output iso path")
                .long("output-iso")
                .takes_value(true))
            .arg(Arg::with_name("extern assets dir")
                .long("extern-assets-dir")
                .takes_value(true))
            .arg(Arg::with_name("profile json path")
                .long("profile")
                .help("Path to JSON file with patch configuration (cli config takes priority). See documentation for details.")
                .takes_value(true))
            .arg(Arg::with_name("force vanilla layout")
                .long("force-vanilla-layout")
                .help("use this to play the vanilla game, but with a custom size factor"))
            .arg(Arg::with_name("qol game breaking")
                .long("qol-game-breaking")
                .help("Fix soft locks and crashes that retro didn't bother addressing"))
            .arg(Arg::with_name("qol cosmetic")
                .long("qol-cosmetic")
                .help("Patch cutscenes to fix continuity errors and UI to improve QoL without affecting IGT or the story"))
            .arg(Arg::with_name("qol cutscenes")
                .long("qol-cutscenes")
                .help("Original, Competitive, Minor, Major")
                .takes_value(true))
            .arg(Arg::with_name("starting room")
                .long("starting-room")
                .help("Room which the player starts their adventure from. Format - <world>:<room name>, where <world> is [Frigate|Tallon|Chozo|Magmoor|Phendrana|Mines|Crater]")
                .takes_value(true))
            .arg(Arg::with_name("starting memo")
                .long("starting-memo")
                .help("String which is shown to the player after they start a new save file")
                .takes_value(true))
            .arg(Arg::with_name("warp to start")
                .long("warp-to-start")
                .help("Allows player to warp to start from any save station"))
            .arg(Arg::with_name("automatic crash screen")
                .long("automatic-crash-screen")
                .help("Makes the crash screen appear without any button combination required"))
            .arg(Arg::with_name("etank capacity")
                .long("etank-capacity")
                .help("Set the etank capacity and base health")
                .takes_value(true))
            .arg(Arg::with_name("nonvaria heat damage")
                .long("nonvaria-heat-damage")
                .help("If the Varia Suit has not been collect, heat damage applies"))
            .arg(Arg::with_name("heat damage per sec")
                .long("heat-damage-per-sec")
                .help("Set the heat damage per seconds spent in a superheated room")
                .takes_value(true))
            .arg(Arg::with_name("staggered suit damage")
                .long("staggered-suit-damage")
                .help(concat!("The suit damage reduction is determinted by the number of suits ",
                                "collected rather than the most powerful one collected.")))
            .arg(Arg::with_name("map default state")
                .long("map-default-state")
                .help("Change the default state of map for each world (Either default, visible or visited)")
                .takes_value(true))
            .arg(Arg::with_name("auto enabled elevators")
                .long("auto-enabled-elevators")
                .help("Every elevator will be automatically enabled without scaning its terminal"))
            .arg(Arg::with_name("artifact hint behavior")
                .long("artifact-hint-behavior")
                .help("Set the behavior of artifact temple hints. Can be 'all', 'none', or 'default' (vanilla)")
                .takes_value(true))
            .arg(Arg::with_name("trilogy disc path")
                .long("flaahgra-music-disc-path")
                .help(concat!("Location of a ISO of Metroid Prime Trilogy. If provided the ",
                                "Flaahgra fight music will be used to replace the original"))
                .takes_value(true))
            .arg(Arg::with_name("suit hue rotate angle")
                .long("suit-hue-rotate-angle")
                .takes_value(true)
                .validator(|s| s.parse::<i32>().map(|_| ())
                                            .map_err(|_| "Expected an integer".to_string())))
            .arg(Arg::with_name("quiet")
                .long("quiet")
                .help("Don't print the progress messages"))
            .arg(Arg::with_name("main menu message")
                .long("main-menu-message")
                .hidden(true)
                .takes_value(true))
            .arg(Arg::with_name("starting items")
                .long("starting-items")
                .takes_value(true)
                .validator(|s| s.parse::<u64>().map(|_| ())
                                            .map_err(|_| "Expected an integer".to_string())))
            .arg(Arg::with_name("item loss items")
                .long("item-loss-items")
                .takes_value(true)
                .validator(|s| s.parse::<u64>().map(|_| ())
                                            .map_err(|_| "Expected an integer".to_string())))
            .arg(Arg::with_name("quickplay")
                .long("quickplay")
                .hidden(true))
            .arg(Arg::with_name("text file comment")
                .long("text-file-comment")
                .hidden(true)
                .takes_value(true))
            .get_matches();

        let mut patch_config = if matches.is_present("profile json path") {
            let json_path = matches.value_of("profile json path").unwrap();
            let cli_json_config_raw: &str = &fs::read_to_string(json_path)
                .map_err(|e| format!("Could not read JSON file: {}", e)).unwrap();

            serde_json::from_str(cli_json_config_raw)
                .map_err(|e| format!("JSON parse failed: {}", e))?
        } else {
            PatchConfigPrivate::default()
        };


        macro_rules! populate_config_bool {
            ($matches:expr; $($name:expr => $cfg:expr,)*) => {
                $(if $matches.is_present($name) {
                    $cfg = Some(true);
                })*
            };
        }

        // bool
        populate_config_bool!(matches;
            "force vanilla layout" => patch_config.force_vanilla_layout,
            "qol game breaking" => patch_config.preferences.qol_game_breaking,
            "qol cosmetic" => patch_config.preferences.qol_cosmetic,
            "qol scans" => patch_config.preferences.qol_pickup_scans,
            "automatic crash screen" => patch_config.preferences.automatic_crash_screen,
            "quickplay" => patch_config.preferences.quickplay,
            "quiet" => patch_config.preferences.quiet,
            "nonvaria heat damage" => patch_config.game_config.nonvaria_heat_damage,
            "staggered suit damage" => patch_config.game_config.staggered_suit_damage,
            "auto enabled elevators" => patch_config.game_config.auto_enabled_elevators,
            "warp to start" => patch_config.game_config.warp_to_start,
        );

        // string
        if let Some(input_iso_path) = matches.value_of("input iso path") {
            patch_config.input_iso  = Some(input_iso_path.to_string());
        }
        if let Some(output_iso_path) = matches.value_of("output iso path") {
            patch_config.output_iso = Some(output_iso_path.to_string());
        }
        if let Some(extern_assets_dir) = matches.value_of("extern assets dir") {
            patch_config.extern_assets_dir = Some(extern_assets_dir.to_string());
        }
        if let Some(map_default_state) = matches.value_of("map default state") {
            patch_config.preferences.map_default_state = Some(map_default_state.to_string());
        }
        if let Some(artifact_behavior) = matches.value_of("artifact hint behavior") {
            patch_config.preferences.artifact_hint_behavior = Some(artifact_behavior.to_string());
        }
        if let Some(trilogy_disc_path) = matches.value_of("trilogy disc path") {
            patch_config.preferences.trilogy_disc_path = Some(trilogy_disc_path.to_string());
        }
        if let Some(starting_room) = matches.value_of("starting room") {
            patch_config.game_config.starting_room = Some(starting_room.to_string());
        }
        if let Some(qol_cutscenes) = matches.value_of("qol cutscenes") {
            patch_config.preferences.qol_cutscenes = Some(qol_cutscenes.to_string());
        }

        // integer/float
        if let Some(s) = matches.value_of("seed") {
            patch_config.seed = Some(s.parse::<u64>().unwrap());
        }
        if let Some(damage) = matches.value_of("heat damage per sec") {
            patch_config.game_config.heat_damage_per_sec = Some(damage.parse::<f32>().unwrap());
        }
        if let Some(etank_capacity) = matches.value_of("etank capacity") {
            patch_config.game_config.etank_capacity = Some(etank_capacity.parse::<u32>().unwrap());
        }
        
        // custom
        if let Some(starting_items_str) = matches.value_of("starting items") {
            patch_config.game_config.starting_items = Some(
                StartingItems::from_u64(starting_items_str.parse::<u64>().unwrap())
            );
        }
        if let Some(item_loss_items_str) = matches.value_of("item loss items") {
            patch_config.game_config.item_loss_items = Some(
                StartingItems::from_u64(item_loss_items_str.parse::<u64>().unwrap())
            );
        }

        patch_config.parse()
    }
}


impl PatchConfigPrivate
{
    fn parse(&self) -> Result<PatchConfig, String>
    {
        let input_iso_path = self.input_iso.as_deref().unwrap_or("prime.iso");
        let input_iso_file = File::open(input_iso_path.trim())
            .map_err(|e| format!("Failed to open {}: {}", input_iso_path, e))?;

        let input_iso = unsafe { memmap::Mmap::map(&input_iso_file) }
            .map_err(|e| format!("Failed to open {}: {}", input_iso_path,  e))?;

        let output_iso_path = self.output_iso.as_deref().unwrap_or("prime_out.iso");

        let output_iso = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&output_iso_path)
            .map_err(|e| format!("Failed to open {}: {}", output_iso_path, e))?;

        let iso_format = if output_iso_path.ends_with(".gcz") {
            IsoFormat::Gcz
        } else if output_iso_path.ends_with(".ciso") {
            IsoFormat::Ciso
        } else {
            IsoFormat::Iso
        };

        let force_vanilla_layout = self.force_vanilla_layout.unwrap_or(false);

        let artifact_hint_behavior = {
            let artifact_hint_behavior_string = self.preferences.artifact_hint_behavior
                .as_deref()
                .unwrap_or("all")
                .trim()
                .to_lowercase();

            if artifact_hint_behavior_string == "all" {
                ArtifactHintBehavior::All
            } else if artifact_hint_behavior_string == "none" {
                ArtifactHintBehavior::None
            } else if artifact_hint_behavior_string == "default" {
                ArtifactHintBehavior::Default
            } else {
                Err(format!(
                    "Unhandled artifact hint behavior - '{}'",
                    artifact_hint_behavior_string
                ))?
            }
        };

        let map_default_state = {
                let map_default_state_string = self.preferences.map_default_state
                                                .as_deref()
                                                .unwrap_or("default")
                                                .trim()
                                                .to_lowercase();
                match &map_default_state_string[..] {
                    "default" => MapState::Default,
                    "visited" => MapState::Visited,
                    "visible" => MapState::Visible,
                    _ => Err(format!(
                        "Unhandled map default state - '{}'",
                        map_default_state_string
                    ))?,
                }
        };

        let flaahgra_music_files = self.preferences.trilogy_disc_path.as_ref()
            .map(|path| extract_flaahgra_music_files(path))
            .transpose()?;

        let item_max_capacity = match &self.game_config.item_max_capacity {
            Some(max_capacity) => {
                max_capacity.iter()
                    .map(|(name, capacity) | (PickupType::from_str(name), *capacity))
                    .collect()
            },
            None => HashMap::new(),
        };

        let qol_game_breaking   = {
            if force_vanilla_layout {
                true
            } else {
                self.preferences.qol_game_breaking.unwrap_or(true)
            }
        };
        let qol_cosmetic        = {
            if force_vanilla_layout {
                false
            } else {
                self.preferences.qol_cosmetic.unwrap_or(true)
            }
        };
        let qol_pickup_scans        = {
            if force_vanilla_layout {
                false
            } else {
                self.preferences.qol_pickup_scans.unwrap_or(true)
            }
        };
        let qol_cutscenes = match self.preferences.qol_cutscenes.as_ref().unwrap_or(&"original".to_string()).to_lowercase().trim() {
            "original" => CutsceneMode::Original,
            "competitive" => CutsceneMode::Competitive,
            "minor" => CutsceneMode::Minor,
            "major" => CutsceneMode::Major,
            _ => panic!("Unknown cutscene mode {}", self.preferences.qol_cutscenes.as_ref().unwrap()),
        };

        let starting_room = {
            if force_vanilla_layout {
                "Frigate:Exterior Docking Hangar".to_string()
            } else {
                self.game_config.starting_room.clone().unwrap_or("Tallon:Landing Site".to_string())
            }
        };

        let starting_items = {
            if force_vanilla_layout {
                StartingItems::from_u64(2188378143)
            } else {
                self.game_config.starting_items.clone().unwrap_or_else(|| StartingItems::from_u64(1))
            }
        };
        
        let warp_to_start   = {
            if force_vanilla_layout {
                false
            } else {
                self.game_config.warp_to_start.unwrap_or(false)
            }
        };

        let main_menu_message = {
            if force_vanilla_layout {
                "".to_string()
            } else {
                self.game_config.main_menu_message.clone().unwrap_or_else(|| "randomprime".to_string())
            }
        };

        let credits_string = {
            if force_vanilla_layout {
                Some("".to_string())
            } else {
                self.game_config.credits_string.clone()
            }
        };

        Ok(PatchConfig {
            input_iso,
            iso_format,
            output_iso,
            force_vanilla_layout,

            seed: self.seed.unwrap_or(123),
            extern_assets_dir: self.extern_assets_dir,

            level_data: self.level_data.clone(),

            qol_game_breaking,
            qol_cosmetic,
            qol_cutscenes,
            qol_pickup_scans,

            phazon_elite_without_dynamo: self.game_config.phazon_elite_without_dynamo.unwrap_or(true), 
            main_plaza_door: self.game_config.main_plaza_door.unwrap_or(true),
            backwards_labs: self.game_config.backwards_labs.unwrap_or(true),
            backwards_frigate: self.game_config.backwards_frigate.unwrap_or(true),
            backwards_upper_mines: self.game_config.backwards_upper_mines.unwrap_or(true),
            backwards_lower_mines: self.game_config.backwards_lower_mines.unwrap_or(true),

            automatic_crash_screen: self.preferences.automatic_crash_screen.unwrap_or(false),
            artifact_hint_behavior,
            flaahgra_music_files,
            suit_hue_rotate_angle: None,
            quiet: self.preferences.quiet.unwrap_or(false),
            quickplay: self.preferences.quickplay.unwrap_or(false),

            starting_room,
            starting_memo: self.game_config.starting_memo.clone(),
            warp_to_start,

            nonvaria_heat_damage: self.game_config.nonvaria_heat_damage.unwrap_or(false),
            staggered_suit_damage: self.game_config.staggered_suit_damage.unwrap_or(false),
            heat_damage_per_sec: self.game_config.heat_damage_per_sec.unwrap_or(10.0),
            auto_enabled_elevators: self.game_config.auto_enabled_elevators.unwrap_or(false),
            multiworld_dol_patches: self.game_config.multiworld_dol_patches.unwrap_or(false),
            update_hint_state_replacement: self.game_config.update_hint_state_replacement.clone(),
            artifact_temple_layer_overrides: self.game_config.artifact_temple_layer_overrides.clone(),
            map_default_state,

            starting_items,
            item_loss_items: self.game_config.item_loss_items.clone()
            .unwrap_or_else(|| StartingItems::from_u64(1)),

            etank_capacity: self.game_config.etank_capacity.unwrap_or(100),
            item_max_capacity: item_max_capacity,

            game_banner: self.game_config.game_banner.clone().unwrap_or_default(),
            comment: self.game_config.comment.clone().unwrap_or(String::new()),
            main_menu_message,

            credits_string,
            artifact_hints: self.game_config.artifact_hints.clone(),

            ctwk_config: self.tweaks.clone(),
        })
    }
}

/*** Helper Methods ***/

pub fn extract_flaahgra_music_files(iso_path: &str) -> Result<[nod_wrapper::FileWrapper; 2], String>
{
    let res = (|| {
        let dw = nod_wrapper::DiscWrapper::new(iso_path)?;
        Ok([
            dw.open_file(CStr::from_bytes_with_nul(b"rui_flaaghraR.dsp\0").unwrap())?,
            dw.open_file(CStr::from_bytes_with_nul(b"rui_flaaghraL.dsp\0").unwrap())?,
        ])
    })();
    res.map_err(|s: String| format!("Failed to extract Flaahgra music files: {}", s))
}
