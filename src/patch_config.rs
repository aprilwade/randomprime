use flate2::{Decompress, FlushDecompress};
use num_bigint::BigUint;
use num_integer::Integer;
use num_traits::ToPrimitive;
use std::collections::HashSet;
use std::{
    borrow::Cow,
    collections::hash_map::DefaultHasher,
    ffi::{CStr, CString},
    hash::{Hash,Hasher},
    convert::TryInto,
    iter,
    fs::{File, OpenOptions},
};

use clap::{
    Arg,
    ArgGroup,
    App,
    Format, // XXX This is an undocumented enum
    crate_version,
};

use serde::{Serialize, Deserialize};

use enum_map::EnumMap;
use crate::elevators::{Elevator, SpawnRoom};
use crate::pickup_meta::PickupType;
use crate::starting_items::StartingItems;

/*** Parsed Config (fn patch_iso) ***/

#[derive(Deserialize)]
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

#[derive(Deserialize, Copy, Clone)]
#[serde(rename_all = "camelCase")]
pub enum ArtifactHintBehavior
{
    Default,
    None,
    All,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GameBanner
{
    pub game_name: Option<String>,
    pub game_name_full: Option<String>,
    pub developer: Option<String>,
    pub developer_full: Option<String>,
    pub description: Option<String>,
}

pub struct PatchConfig
{
    pub input_iso: memmap::Mmap,
    pub iso_format: IsoFormat,
    pub output_iso: File,

    pub layout: Layout,
    
    pub skip_frigate: bool,
    pub skip_hudmenus: bool,
    pub keep_fmvs: bool,
    pub obfuscate_items: bool,
    pub etank_capacity: u32,
    pub nonvaria_heat_damage: bool,
    pub heat_damage_per_sec: f32,
    pub staggered_suit_damage: bool,
    pub max_obtainable_missiles: u32,
    pub max_obtainable_power_bombs: u32,
    pub auto_enabled_elevators: bool,
    pub quiet: bool,

    pub starting_items: StartingItems,
    pub random_starting_items: StartingItems,

    pub enable_vault_ledge_door: bool,
    pub artifact_hint_behavior: ArtifactHintBehavior,

    pub flaahgra_music_files: Option<[nod_wrapper::FileWrapper; 2]>,

    pub suit_hue_rotate_angle: Option<i32>,

    pub quickplay: bool,

    pub game_banner: GameBanner,
    pub comment: String,
    pub main_menu_message: String,
}

/*** Un-Parsed Config ***/

#[derive(Deserialize, Clone)]
#[serde(untagged)]
enum LayoutWrapper
{
    String(String),
    Struct {
        pickups: Vec<PickupType>,
        starting_location: SpawnRoom,
        // #[serde(default = "Elevator::default_layout")]
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Preferences
{
    skip_hudmenus: Option<bool>,
    obfuscate_items: Option<bool>,
    artifact_hint_behavior: Option<String>,
    trilogy_disc_path: Option<String>,
    keep_fmvs: Option<bool>,
    quickplay: Option<bool>,
    quiet: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GameConfig
{
    skip_frigate: Option<bool>, // TODO: remove, calculate automatically once starting room is a thing
    nonvaria_heat_damage: Option<bool>,
    staggered_suit_damage: Option<bool>,
    heat_damage_per_sec: Option<f32>,
    auto_enabled_elevators: Option<bool>,
    enable_vault_ledge_door: Option<bool>, // TODO: remove, calculate automatically once door patching is a thing

    starting_items: Option<StartingItems>,
    random_starting_items: Option<StartingItems>, // TODO: replace with a "game start memo" string

    etank_capacity: Option<u32>,
    max_obtainable_missiles: Option<u32>, // TODO: rename
    max_obtainable_power_bombs: Option<u32>, // TODO: rename

    game_banner: Option<GameBanner>,
    comment: Option<String>,
    main_menu_message: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PatchConfigPrivate
{
    input_iso: Option<String>,
    output_iso: Option<String>,
    iso_format: Option<IsoFormat>,
    preferences: Option<Preferences>,
    game_config: Option<GameConfig>,
    layout: Option<LayoutWrapper>, // TODO: only support struct (because of doors)
}

/** Command Line Interface ***/

/*
fn cli_parse(matches: &App) -> Result<PatchConfig, String>
{
    Ok(PatchConfig {
        input_iso: input_iso_mmap,
        output_iso: out_iso,

        layout,

        iso_format,
        skip_hudmenus: matches.is_present("skip hudmenus"),
        skip_frigate: matches.is_present("skip frigate"),
        etank_capacity: matches.value_of("etank capacity")
                                    .unwrap_or_default()
                                    .parse::<u32>()
                                    .unwrap_or(100),
        nonvaria_heat_damage: matches.is_present("nonvaria heat damage"),
        heat_damage_per_sec: matches.value_of("heat damage per sec")
                                    .unwrap_or_default()
                                    .parse::<f32>()
                                    .unwrap_or(10.0),
        staggered_suit_damage: matches.is_present("staggered suit damage"),
        max_obtainable_missiles: matches.value_of("max obtainable missiles")
                                    .unwrap_or_default()
                                    .parse::<u32>()
                                    .unwrap_or(250),
        max_obtainable_power_bombs: matches.value_of("max obtainable power bombs")
                                    .unwrap_or_default()
                                    .parse::<u32>()
                                    .unwrap_or(8),
        keep_fmvs: matches.is_present("keep attract mode"),
        obfuscate_items: matches.is_present("obfuscate items"),
        auto_enabled_elevators: matches.is_present("auto enabled elevators"),
        quiet: matches.is_present("quiet"),
        enable_vault_ledge_door: matches.is_present("enable vault ledge door"),

        artifact_hint_behavior,

        flaahgra_music_files,
        suit_hue_rotate_angle: matches.value_of("suit hue rotate angle")
                .map(|s| s.parse::<i32>().unwrap()),

        // XXX We can unwrap safely because we verified the parse earlier
        starting_items: matches.value_of("change starting items")
                                .map(|s| StartingItems::from_u64(s.parse().unwrap()))
                                .unwrap_or_default(),
        random_starting_items,

        comment: matches.value_of("text file comment").unwrap_or("").to_string(),
        main_menu_message: matches.value_of("main menu message").unwrap_or("").to_string(),

        quickplay: matches.is_present("quickplay"),

        bnr_game_name: None,
        bnr_developer: None,

        bnr_game_name_full: None,
        bnr_developer_full: None,
        bnr_description: None,
    })
}
*/

/*** Define Defaults ***/

const DEFAULT_GAME_BANNER: GameBanner = GameBanner {
    game_name: None,
    game_name_full: None,
    developer: None,
    developer_full: None,
    description: None,
};

const DEFAULT_GAME_CONFIG: GameConfig = GameConfig {
    skip_frigate: None,
    nonvaria_heat_damage: None,
    staggered_suit_damage: None,
    heat_damage_per_sec: None,
    auto_enabled_elevators: None,
    enable_vault_ledge_door: None,
    starting_items: None,
    random_starting_items: None,
    etank_capacity: None,
    max_obtainable_missiles: None,
    max_obtainable_power_bombs: None,
    game_banner: None,
    comment: None,
    main_menu_message: None,
};

/*
const DEFAULT_PREFERENCES: Preferences = Preferences {

};
*/

const DEFAULT_PATCHER_CONFIG: PatchConfigPrivate = PatchConfigPrivate {
    input_iso: None,
    output_iso: None,
    iso_format: None,
    game_config: None,
    preferences: None,
    layout: None,
};

/*** Parse Patcher Input ***/
pub fn randomprime_parse_input(
    json_config_raw:Option<&str>,
    cli: bool,
)
    -> Result<PatchConfig, String>
{
    // Start with default config
    let mut patch_config = DEFAULT_PATCHER_CONFIG;

    // 1st - Parse JSON
    /*
    if json_config_raw.is_some()
    {
        let mut json_config: PatchConfig = serde_json::from_str(&json_config_raw.unwrap())
            .map_err(|e| format!("JSON parse failed: {}", e))?;
    }
    */

    // 2nd - Parse cli (cli has priority over JSON)
    /*
    if cli {
        App::new("randomprime ISO patcher")
            .version(crate_version!())
            .arg(Arg::with_name("input iso path")
                .long("input-iso")
                .required(true)
                .takes_value(true))
            .arg(Arg::with_name("output iso path")
                .long("output-iso")
                .required(true)
                .takes_value(true))
            .arg(Arg::with_name("pickup layout")
                .long("layout")
                .required(true)
                .takes_value(true)
                .allow_hyphen_values(true))
            .arg(Arg::with_name("skip frigate")
                .long("skip-frigate")
                .help("New save files will skip the \"Space Pirate Frigate\" tutorial level"))
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
            .arg(Arg::with_name("max obtainable missiles")
                .long("max-obtainable-missiles")
                .help("Set the max amount of Missiles you can carry")
                .takes_value(true))
            .arg(Arg::with_name("max obtainable power bombs")
                .long("max-obtainable-power-bombs")
                .help("Set the max amount of Power Bombs you can carry")
                .takes_value(true))
            .arg(Arg::with_name("auto enabled elevators")
                .long("auto-enabled-elevators")
                .help("Every elevator will be automatically enabled without scaning its terminal"))
            .arg(Arg::with_name("skip impact crater")
                .long("skip-impact-crater")
                .help("Elevators to the Impact Crater immediately go to the game end sequence"))
            .arg(Arg::with_name("enable vault ledge door")
                .long("enable-vault-ledge-door")
                .help("Enable Chozo Ruins Vault door from Main Plaza"))
    
            .arg(Arg::with_name("all artifact hints")
                .long("all-artifact-hints")
                .help("All artifact location hints are available immediately"))
            .arg(Arg::with_name("no artifact hints")
                .long("no-artifact-hints")
                .help("Artifact location hints are disabled"))
            .group(ArgGroup::with_name("artifact hint behavior")
                   .args(&["all artifact hints", "no artifact hints"]))
    
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
            .arg(Arg::with_name("random starting items")
                .long("random-starting-items")
                .hidden(true)
                .takes_value(true)
                .validator(|s| s.parse::<u64>().map(|_| ())
                                            .map_err(|_| "Expected an integer".to_string())))
            .arg(Arg::with_name("change starting items")
                .long("starting-items")
                .hidden(true)
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



    } */

    patch_config.parse()
}

impl PatchConfigPrivate
{
    fn parse(&self) -> Result<PatchConfig, String>
    {
        let preferences = self.preferences.as_ref().unwrap();
        let game_config = self.game_config.as_ref().unwrap();
        
        let input_iso_path = self.input_iso.as_ref().unwrap();
        let input_iso_file = File::open(input_iso_path.trim())
            .map_err(|e| format!("Failed to open {}: {}", input_iso_path, e))?;

        let input_iso = unsafe { memmap::Mmap::map(&input_iso_file) }
            .map_err(|e| format!("Failed to open {}: {}", input_iso_path,  e))?;

        let output_iso_path = self.output_iso.as_ref().unwrap();

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

        let _layout = self.layout.as_ref().unwrap().clone();
        let layout = _layout.try_into()?;

        let artifact_hint_behavior = {
            let artifact_hint_behavior_string = preferences.artifact_hint_behavior.as_ref().unwrap().trim().to_lowercase();

            if artifact_hint_behavior_string == "all" {
                ArtifactHintBehavior::All
            } else if artifact_hint_behavior_string == "none" {
                ArtifactHintBehavior::None  
            } else if artifact_hint_behavior_string == "default" {
                ArtifactHintBehavior::Default
            } else {
                panic!("Unhandled artifact hint behavior - '{}'", artifact_hint_behavior_string);
            }
        };
    
        let flaahgra_music_files = if let Some(path) = preferences.trilogy_disc_path.as_ref() {
            Some(extract_flaahgra_music_files(&path)?)
        } else {
            None
        };

        Ok(PatchConfig {
            input_iso,
            iso_format,
            output_iso,
            layout,

            skip_hudmenus: preferences.skip_hudmenus.unwrap(),
            obfuscate_items: preferences.obfuscate_items.unwrap(),
            artifact_hint_behavior,
            flaahgra_music_files,
            keep_fmvs: preferences.keep_fmvs.unwrap(),
            suit_hue_rotate_angle: None,
            quiet: preferences.quiet.unwrap(),
            quickplay: preferences.quickplay.unwrap(),

            skip_frigate: game_config.skip_frigate.unwrap(),
            nonvaria_heat_damage: game_config.nonvaria_heat_damage.unwrap(),
            staggered_suit_damage: game_config.staggered_suit_damage.unwrap(),
            heat_damage_per_sec: game_config.heat_damage_per_sec.unwrap(),
            auto_enabled_elevators: game_config.auto_enabled_elevators.unwrap(),
            enable_vault_ledge_door: game_config.enable_vault_ledge_door.unwrap(),

            starting_items: game_config.starting_items.as_ref().unwrap().clone(),
            random_starting_items: game_config.random_starting_items.as_ref().unwrap().clone(),

            etank_capacity: game_config.etank_capacity.unwrap(),
            max_obtainable_missiles: game_config.max_obtainable_missiles.unwrap(),
            max_obtainable_power_bombs: game_config.max_obtainable_power_bombs.unwrap(),

            game_banner: game_config.game_banner.as_ref().unwrap().clone(),
            comment: game_config.comment.as_ref().unwrap().to_string(),
            main_menu_message: game_config.main_menu_message.as_ref().unwrap().to_string(),
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

        // XXX The distribution on this hash probably isn't very good, but we don't use it for anything
        //     particularly important anyway...
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
