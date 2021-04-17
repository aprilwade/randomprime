use num_bigint::BigUint;
use num_integer::Integer;
use num_traits::ToPrimitive;
use std::{
    collections::hash_map::DefaultHasher,
    ffi::CStr,
    hash::{Hash, Hasher},
    convert::TryInto,
    collections::HashMap,
    iter,
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

use enum_map::EnumMap;
use crate::elevators::{Elevator, SpawnRoom};
use crate::pickup_meta::PickupType;
use crate::starting_items::StartingItems;

/*** Parsed Config (fn patch_iso) ***/

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum IsoFormat
{
    Iso,
    Gcz,
    Ciso,
}

#[derive(Clone, Debug)]
pub struct Layout
{
    pub pickups: Vec<PickupType>,
    pub starting_location: SpawnRoom,
    pub elevators: EnumMap<Elevator, SpawnRoom>,
    pub seed: u64,
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

// TODO: defaults
#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PickupConfig
{
    // pub pickup_type: String,
    // pub count: u32,
    // pub model: PickupModelType,
    // pub scan_text: String,
    // pub hudmemo_text: String,
    // pub desination: String,
    // pub position: [f32;3],
}

// TODO: defaults
#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RoomConfig
{
    // pub remove_locks: bool,
    // pub superheated: bool,
    // pub remove_water: bool,
    // pub submerge: bool,
    // pub extra_water: Vec<WaterConfig>,
    // pub doors: Vec<String>,
    // pub blast_shields: Vec<String>,
    // pub pickups: Vec<PickupConfig>,
    // pub extra_pickups: Vec<PickupConfig>,
    // pub extra_scans: Vec<ScanConfig>,
    // pub aether_transform: Vec<AetherTransformConfig>,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LevelConfig
{
    pub transports: HashMap<String, String>,
    pub rooms: HashMap<String, RoomConfig>,
}

#[derive(Debug)]
pub struct PatchConfig
{
    pub input_iso: memmap::Mmap,
    pub iso_format: IsoFormat,
    pub output_iso: File,

    pub layout: Layout,

    pub level_data: HashMap<String, LevelConfig>,

    pub starting_room: String,
    pub starting_memo: Option<String>,

    pub skip_hudmenus: bool,
    pub keep_fmvs: bool,
    pub obfuscate_items: bool,
    pub etank_capacity: u32,
    pub nonvaria_heat_damage: bool,
    pub heat_damage_per_sec: f32,
    pub staggered_suit_damage: bool,
    pub missile_capacity: u32,
    pub power_bomb_capacity: u32,
    pub map_default_state: MapState,
    pub auto_enabled_elevators: bool,
    pub quiet: bool,

    pub starting_items: StartingItems,
    pub item_loss_items: StartingItems,

    pub enable_vault_ledge_door: bool,
    pub artifact_hint_behavior: ArtifactHintBehavior,

    pub flaahgra_music_files: Option<[nod_wrapper::FileWrapper; 2]>,

    pub suit_hue_rotate_angle: Option<i32>,

    pub quickplay: bool,

    pub game_banner: GameBanner,
    pub comment: String,
    pub main_menu_message: String,
}


/*** Un-Parsed Config (doubles as JSON input specification) ***/

#[derive(Deserialize, Clone, Debug)]
#[serde(untagged)]
enum LayoutWrapper
{
    String(String),
    Struct {
        pickups: Vec<PickupType>,
        starting_location: SpawnRoom,
        elevators: EnumMap<Elevator, SpawnRoom>,
    },
}

impl TryInto<Layout> for LayoutWrapper
{
    type Error = String;
    fn try_into(self) -> Result<Layout, Self::Error>
    {
        match self {
            LayoutWrapper::String(s) => s.parse(),
            LayoutWrapper::Struct { pickups, starting_location, elevators } => {
                let mut hasher = DefaultHasher::new();
                pickups.hash(&mut hasher);
                starting_location.hash(&mut hasher);
                elevators.hash(&mut hasher);
                Ok(Layout {
                    pickups,
                    starting_location,
                    elevators,
                    seed: hasher.finish(),
                })
            },
        }
    }
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
struct Preferences
{
    skip_hudmenus: Option<bool>,
    obfuscate_items: Option<bool>,
    map_default_state: Option<String>,
    artifact_hint_behavior: Option<String>,
    trilogy_disc_path: Option<String>,
    keep_fmvs: Option<bool>,
    quickplay: Option<bool>,
    quiet: Option<bool>,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
struct GameConfig
{
    starting_room: Option<String>,
    starting_memo: Option<String>,

    nonvaria_heat_damage: Option<bool>,
    staggered_suit_damage: Option<bool>,
    heat_damage_per_sec: Option<f32>,
    auto_enabled_elevators: Option<bool>,
    enable_vault_ledge_door: Option<bool>, // TODO: remove, calculate automatically once door patching is a thing

    starting_items: Option<StartingItems>,
    item_loss_items: Option<StartingItems>,

    etank_capacity: Option<u32>,
    missile_capacity: Option<u32>,
    power_bomb_capacity: Option<u32>,

    game_banner: Option<GameBanner>,
    comment: Option<String>,
    main_menu_message: Option<String>,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
struct PatchConfigPrivate
{
    input_iso: Option<String>,
    output_iso: Option<String>,

    #[serde(default)]
    preferences: Preferences,

    #[serde(default)]
    game_config: GameConfig,

    #[serde(default)]
    level_data: HashMap<String, LevelConfig>,

    layout: Option<LayoutWrapper>, // TODO: only support struct (because of doors)
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
            .arg(Arg::with_name("profile json path")
                .long("profile")
                .help("Path to JSON file with patch configuration (cli config takes priority). See documentation for details.")
                .takes_value(true))
            .arg(Arg::with_name("pickup layout")
                .long("layout")
                .takes_value(true)
                .allow_hyphen_values(true))
            .arg(Arg::with_name("starting room")
                .long("starting-room")
                .help("Room which the player starts their adventure from. Format - <world>:<room name>, where <world> is [Frigate|Tallon|Chozo|Magmoor|Phendrana|Mines|Crater]")
                .takes_value(true))
            .arg(Arg::with_name("starting memo")
                .long("starting-memo")
                .help("String which is shown to the player after they start a new save file")
                .takes_value(true))
            .arg(Arg::with_name("skip hudmenus")
                .long("non-modal-item-messages")
                .help("Display a non-modal message when an item is is acquired"))
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
            .arg(Arg::with_name("missile capacity")
                .long("missile-capacity")
                .help("Set the max amount of Missiles you can carry")
                .takes_value(true))
            .arg(Arg::with_name("power bomb capacity")
                .long("power-bomb-capacity")
                .help("Set the max amount of Power Bombs you can carry")
                .takes_value(true))
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
            .arg(Arg::with_name("enable vault ledge door")
                .long("enable-vault-ledge-door")
                .help("Enable Chozo Ruins Vault door from Main Plaza"))
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
            .arg(Arg::with_name("keep attract mode")
                .long("keep-attract-mode")
                .help("Keeps the attract mode FMVs, which are removed by default"))
            .arg(Arg::with_name("obfuscate items")
                .long("obfuscate-items")
                .help("Replace all item models with an obfuscated one"))
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
            "skip hudmenus" => patch_config.preferences.skip_hudmenus,
            "obfuscate items" => patch_config.preferences.obfuscate_items,
            "keep attract mode" => patch_config.preferences.keep_fmvs,
            "quickplay" => patch_config.preferences.quickplay,
            "quiet" => patch_config.preferences.quiet,
            "nonvaria heat damage" => patch_config.game_config.nonvaria_heat_damage,
            "staggered suit damage" => patch_config.game_config.staggered_suit_damage,
            "auto enabled elevators" => patch_config.game_config.auto_enabled_elevators,
            "enable vault ledge door" => patch_config.game_config.enable_vault_ledge_door,
        );

        // string
        if let Some(input_iso_path) = matches.value_of("input iso path") {
            patch_config.input_iso  = Some(input_iso_path.to_string());
        }
        if let Some(output_iso_path) = matches.value_of("output iso path") {
            patch_config.output_iso = Some(output_iso_path.to_string());
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

        // integer/float
        if let Some(damage) = matches.value_of("heat damage per sec") {
            patch_config.game_config.heat_damage_per_sec = Some(damage.parse::<f32>().unwrap());
        }
        if let Some(etank_capacity) = matches.value_of("etank capacity") {
            patch_config.game_config.etank_capacity = Some(etank_capacity.parse::<u32>().unwrap());
        }
        if let Some(s) = matches.value_of("missile capacity") {
            patch_config.game_config.missile_capacity= Some(s.parse::<u32>().unwrap());
        }
        if let Some(s) = matches.value_of("power bomb capacity") {
            patch_config.game_config.power_bomb_capacity = Some(s.parse::<u32>().unwrap());
        }

        // custom
        if let Some(pickup_layout_str) = matches.value_of("pickup layout") {
            patch_config.layout = Some(LayoutWrapper::String(pickup_layout_str.to_string()));
        }
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

        let _layout = self.layout.clone()
            .unwrap_or_else(|| LayoutWrapper::String(
                "NCiq7nTAtTnqPcap9VMQk_o8Qj6ZjbPiOdYDB5tgtwL_f01-UpYklNGnL-gTu5IeVW3IoUiflH5LqNXB3wVEER4".to_string()
            ));

        let layout = _layout.try_into()?;

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

        Ok(PatchConfig {
            input_iso,
            iso_format,
            output_iso,
            layout,
            level_data: self.level_data.clone(),

            skip_hudmenus: self.preferences.skip_hudmenus.unwrap_or(true),
            obfuscate_items: self.preferences.obfuscate_items.unwrap_or(false),
            artifact_hint_behavior,
            flaahgra_music_files,
            keep_fmvs: self.preferences.keep_fmvs.unwrap_or(false),
            suit_hue_rotate_angle: None,
            quiet: self.preferences.quiet.unwrap_or(false),
            quickplay: self.preferences.quickplay.unwrap_or(false),

            starting_room: self.game_config.starting_room.clone().unwrap_or("Tallon:Landing Site".to_string()),
            starting_memo: self.game_config.starting_memo.clone(),

            nonvaria_heat_damage: self.game_config.nonvaria_heat_damage.unwrap_or(false),
            staggered_suit_damage: self.game_config.staggered_suit_damage.unwrap_or(false),
            heat_damage_per_sec: self.game_config.heat_damage_per_sec.unwrap_or(10.0),
            auto_enabled_elevators: self.game_config.auto_enabled_elevators.unwrap_or(false),
            map_default_state,
            enable_vault_ledge_door: self.game_config.enable_vault_ledge_door.unwrap_or(false),

            starting_items: self.game_config.starting_items.clone()
            .unwrap_or_else(|| StartingItems::from_u64(1)),
            item_loss_items: self.game_config.item_loss_items.clone()
            .unwrap_or_else(|| StartingItems::from_u64(1)),

            etank_capacity: self.game_config.etank_capacity.unwrap_or(100),
            missile_capacity: self.game_config.missile_capacity.unwrap_or(999),
            power_bomb_capacity: self.game_config.power_bomb_capacity.unwrap_or(8),

            game_banner: self.game_config.game_banner.clone().unwrap_or_default(),
            comment: self.game_config.comment.clone().unwrap_or(String::new()),
            main_menu_message: self.game_config.main_menu_message.clone()
                .unwrap_or_else(|| "randomprime".to_string()),
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

pub fn parse_layout_chars_to_ints<I>(bytes: &[u8], layout_data_size: usize, checksum_size: usize, is: I)
    -> Result<Vec<u8>, String>
    where I: Iterator<Item = u8> + Clone
{
    const LAYOUT_CHAR_TABLE: [u8; 64] =
        *b"ABCDEFGHIJKLMNOPQRSTUWVXYZabcdefghijklmnopqrstuwvxyz0123456789-_";

    let mut sum: BigUint = 0u8.into();
    for c in bytes.iter().rev() {
        if let Some(idx) = LAYOUT_CHAR_TABLE.iter().position(|i| i == c) {
            sum = sum * BigUint::from(64u8) + BigUint::from(idx);
        } else {
            return Err(format!("Layout contains invalid character '{}'.", c));
        }
    }

    // Reverse the order of the odd bits
    let mut bits = sum.to_str_radix(2).into_bytes();
    for i in 0..(bits.len() / 4) {
        let len = bits.len() - bits.len() % 2;
        bits.swap(i * 2 + 1, len - i * 2 - 1);
    }
    sum = BigUint::parse_bytes(&bits, 2).unwrap();

    // The upper `checksum_size` bits are a checksum, so seperate them from the sum.
    let checksum_bitmask = (1u8 << checksum_size) - 1;
    let checksum = sum.clone() & (BigUint::from(checksum_bitmask) << layout_data_size);
    sum -= checksum.clone();
    let checksum = (checksum >> layout_data_size).to_u8().unwrap();

    let mut computed_checksum = 0;
    {
        let mut sum = sum.clone();
        while sum > 0u8.into() {
            let remainder = (sum.clone() & BigUint::from(checksum_bitmask)).to_u8().unwrap();
            computed_checksum = (computed_checksum + remainder) & checksum_bitmask;
            sum >>= checksum_size;
        }
    }
    if checksum != computed_checksum {
        return Err("Layout checksum failed.".to_string());
    }

    let mut res = vec![];
    for denum in is {
        let (quotient, remainder) = sum.div_rem(&denum.into());
        res.push(remainder.to_u8().unwrap());
        sum = quotient;
    }

    assert!(sum == 0u8.into());

    res.reverse();
    Ok(res)
}

impl std::str::FromStr for Layout
{
    type Err = String;
    fn from_str(text: &str) -> Result<Layout, String>
    {
        if !text.is_ascii() {
            return Err("Layout string contains non-ascii characters.".to_string());
        }
        let text = text.as_bytes();

        let (elevator_bytes, pickup_bytes) = if let Some(n) = text.iter().position(|c| *c == b'.') {
            (&text[..n], &text[(n + 1)..])
        } else {
            (b"qzoCAr2fwehJmRjM" as &[u8], text)
        };

        if elevator_bytes.len() != 16 {
            let msg = "The section of the layout string before the '.' should be 16 characters";
            return Err(msg.to_string());
        }

        let (pickup_bytes, has_scan_visor) = if pickup_bytes.starts_with(b"!") {
            (&pickup_bytes[1..], true)
        } else {
            (pickup_bytes, false)
        };
        if pickup_bytes.len() != 87 {
            return Err("Layout string should be exactly 87 characters".to_string());
        }

        // XXX The distribution on this hash probably isn't very good, but we don't use it for
        //     anything particularly important anyway...
        let mut hasher = DefaultHasher::new();
        hasher.write(elevator_bytes);
        hasher.write(pickup_bytes);
        let seed = hasher.finish();

        let pickup_layout = parse_layout_chars_to_ints(
                pickup_bytes,
                if has_scan_visor { 521 } else { 517 },
                if has_scan_visor { 1 } else { 5 },
                iter::repeat(if has_scan_visor { 37u8 } else { 36u8 }).take(100)
            ).map_err(|err| format!("Parsing pickup layout: {}", err))?;
        let pickups = pickup_layout.iter()
            .map(|i| PickupType::from_idx(*i as usize).unwrap())
            .collect();

        let elevator_nums = parse_layout_chars_to_ints(
                elevator_bytes,
                91, 5,
                iter::once(21u8).chain(iter::repeat(20u8).take(20))
            ).map_err(|err| format!("Parsing elevator layout: {}", err))?;

        let starting_location = SpawnRoom::from_u32(*elevator_nums.last().unwrap() as u32)
            .unwrap();
        let mut elevators = EnumMap::<Elevator, SpawnRoom>::new();
        elevators.extend(elevator_nums[..(elevator_nums.len() - 1)].iter()
            .zip(Elevator::iter())
            .map(|(i, elv)| (elv, SpawnRoom::from_u32(*i as u32).unwrap()))
        );

        Ok(Layout {
            pickups,
            starting_location,
            elevators,
            seed,
        })
    }
}
