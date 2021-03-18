use num_bigint::BigUint;
use num_integer::Integer;
use num_traits::ToPrimitive;
use std::{
    collections::hash_map::DefaultHasher,
    ffi::{CStr},
    hash::{Hash,Hasher},
    convert::TryInto,
    iter,
    fs::{File, OpenOptions},
    fs,
};

use clap::{
    Arg,
    App,
    crate_version,
};

use serde::{Deserialize};

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

#[derive(Debug)]
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

/*** Define Defaults (all None) ***/

fn default_patch_config_private()
    -> PatchConfigPrivate
{
    PatchConfigPrivate {
        input_iso: None,
        output_iso: None,
        game_config: default_game_config(),
        preferences: default_preferences(),
        layout: None,
    }
}

fn default_preferences()
    -> Preferences
{
    Preferences {
        skip_hudmenus: None,
        obfuscate_items: None,
        artifact_hint_behavior: None,
        trilogy_disc_path: None,
        keep_fmvs: None,
        quickplay: None,
        quiet: None,
    }
}

fn default_game_config()
    -> GameConfig
{
    GameConfig {
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
    }
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

#[derive(Deserialize, Debug)]
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

#[derive(Deserialize, Debug)]
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

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct PatchConfigPrivate
{
    input_iso: Option<String>,
    output_iso: Option<String>,

    #[serde(default = "default_preferences")]
    preferences: Preferences,
    
    #[serde(default = "default_game_config")]
    game_config: GameConfig,
    
    layout: Option<LayoutWrapper>, // TODO: only support struct (because of doors)
}

/*** Parse Patcher Input ***/

pub fn randomprime_parse_input(
    json_config_raw: Option<&str>,
    cli: bool,
)
    -> Result<PatchConfig, String>
{
    // 0th pass - Start with default config
    let mut patch_config = default_patch_config_private();

    // 1st pass - Parse c-interface JSON
    if json_config_raw.is_some()
    {
        let json_config: PatchConfigPrivate = serde_json::from_str(json_config_raw.unwrap())
            .map_err(|e| format!("JSON parse failed: {}", e))?;
        
        merge_config(&mut patch_config, &json_config);
    }

    if cli
    {
        let cli_app = App::new("randomprime ISO patcher")
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
            .arg(Arg::with_name("artifact hint behavior")
                .long("artifact-hint-behavior")
                .help("Set the behavior of artifact temple hints. Can be 'all', 'none', or 'default' (vanilla)")
                .takes_value(true))
            .arg(Arg::with_name("skip impact crater")
                .long("skip-impact-crater")
                .help("Elevators to the Impact Crater immediately go to the game end sequence"))
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

        // 2nd pass - Parse JSON file specified by cli
        if cli_app.is_present("profile json path")
        {
            let json_path = cli_app.value_of("profile json path").unwrap();
            let cli_json_config_raw:&str = &fs::read_to_string(json_path)
                        .map_err(|e| format!("Could not read JSON file: {}",e)).unwrap();

            let cli_json_config: PatchConfigPrivate = serde_json::from_str(cli_json_config_raw)
                .map_err(|e| format!("JSON parse failed: {}", e))?;

            merge_config(&mut patch_config, &cli_json_config);
        }

        // 3rd pass - Parse cli options (cli takes priority over JSON, so we parse last)
        {
            // TODO: prime realestate for some macros, a shame I'm too dumb to figure it out - toasterparty
            // TODO: error handling on unwrap/parse

            // string
            if cli_app.is_present("input iso path" ) {patch_config.input_iso  = Some(cli_app.value_of("input iso path" ).unwrap().to_string());}
            if cli_app.is_present("output iso path") {patch_config.output_iso = Some(cli_app.value_of("output iso path").unwrap().to_string());}
            if cli_app.is_present("artifact hint behavior") {patch_config.preferences.artifact_hint_behavior = Some(cli_app.value_of("artifact hint behavior").unwrap().to_string());}
            if cli_app.is_present("trilogy disc path"     ) {patch_config.preferences.trilogy_disc_path      = Some(cli_app.value_of("trilogy disc path"     ).unwrap().to_string());}

            // bool
            if cli_app.is_present("skip hudmenus"          ) {patch_config.preferences.skip_hudmenus           = Some(true);}
            if cli_app.is_present("obfuscate items"        ) {patch_config.preferences.obfuscate_items         = Some(true);}
            if cli_app.is_present("keep attract mode"      ) {patch_config.preferences.keep_fmvs               = Some(true);}
            if cli_app.is_present("quickplay"              ) {patch_config.preferences.quickplay               = Some(true);}
            if cli_app.is_present("quiet"                  ) {patch_config.preferences.quiet                   = Some(true);}
            if cli_app.is_present("skip frigate"           ) {patch_config.game_config.skip_frigate            = Some(true);}
            if cli_app.is_present("nonvaria heat damage"   ) {patch_config.game_config.nonvaria_heat_damage    = Some(true);}
            if cli_app.is_present("staggered suit damage"  ) {patch_config.game_config.staggered_suit_damage   = Some(true);}
            if cli_app.is_present("auto enabled elevators" ) {patch_config.game_config.auto_enabled_elevators  = Some(true);}
            if cli_app.is_present("enable vault ledge door") {patch_config.game_config.enable_vault_ledge_door = Some(true);}

            // integer/float
            if cli_app.is_present("heat damage per sec"       ) {patch_config.game_config.heat_damage_per_sec        = Some(cli_app.value_of("heat damage per sec"       ).unwrap().parse::<f32>().unwrap());}
            if cli_app.is_present("etank capacity"            ) {patch_config.game_config.etank_capacity             = Some(cli_app.value_of("etank capacity"            ).unwrap().parse::<u32>().unwrap());}
            if cli_app.is_present("max obtainable missiles"   ) {patch_config.game_config.max_obtainable_missiles    = Some(cli_app.value_of("max obtainable missiles"   ).unwrap().parse::<u32>().unwrap());}
            if cli_app.is_present("max obtainable power bombs") {patch_config.game_config.max_obtainable_power_bombs = Some(cli_app.value_of("max obtainable power bombs").unwrap().parse::<u32>().unwrap());}

            // custom
            if cli_app.is_present("pickup layout")
            {
                patch_config.layout  = Some(
                    LayoutWrapper::String(
                        cli_app.value_of("pickup layout")
                        .unwrap()
                        .to_string()
                    )
                );
            }
            if cli_app.is_present("starting items")
            {
                patch_config.game_config.starting_items = Some(
                    StartingItems::from_u64(
                        cli_app.value_of("starting items")
                            .unwrap()
                            .parse::<u64>()
                            .unwrap()
                    )
                );
            }
            if cli_app.is_present("random starting items")
            {
                patch_config.game_config.random_starting_items = Some(
                    StartingItems::from_u64(
                        cli_app.value_of("random starting items")
                            .unwrap()
                            .parse::<u64>()
                            .unwrap()
                    )
                );
            }

            // TODO: missing banner, comment and main menu message
        }
    }

    // 4th pass - set any remaining unspecifed config values with sensible defaults
    // TODO: prime realestate for some macros, a shame I'm too dumb to figure it out - toasterparty
    if patch_config.input_iso.is_none()                                {patch_config.input_iso                                = Some("prime.iso".to_string());}
    if patch_config.output_iso.is_none()                               {patch_config.output_iso                               = Some("prime_out.iso".to_string());}
    if patch_config.preferences.skip_hudmenus.is_none()                {patch_config.preferences.skip_hudmenus                = Some(true);}
    if patch_config.preferences.obfuscate_items.is_none()              {patch_config.preferences.obfuscate_items              = Some(false);}
    if patch_config.preferences.artifact_hint_behavior.is_none()       {patch_config.preferences.artifact_hint_behavior       = Some("all".to_string());}
    // trilogy disc path stays None so that it gets skipped
    if patch_config.preferences.keep_fmvs.is_none()                    {patch_config.preferences.keep_fmvs                    = Some(false);}
    if patch_config.preferences.quickplay.is_none()                    {patch_config.preferences.quickplay                    = Some(false);}
    if patch_config.preferences.quiet.is_none()                        {patch_config.preferences.quiet                        = Some(false);}
    if patch_config.game_config.skip_frigate.is_none()                 {patch_config.game_config.skip_frigate                 = Some(true);}
    if patch_config.game_config.nonvaria_heat_damage.is_none()         {patch_config.game_config.nonvaria_heat_damage         = Some(false);}
    if patch_config.game_config.staggered_suit_damage.is_none()        {patch_config.game_config.staggered_suit_damage        = Some(false);}
    if patch_config.game_config.heat_damage_per_sec.is_none()          {patch_config.game_config.heat_damage_per_sec          = Some(10.0);}
    if patch_config.game_config.auto_enabled_elevators.is_none()       {patch_config.game_config.auto_enabled_elevators       = Some(false);}
    if patch_config.game_config.enable_vault_ledge_door.is_none()      {patch_config.game_config.enable_vault_ledge_door      = Some(false);}
    if patch_config.game_config.starting_items.is_none()               {patch_config.game_config.starting_items               = Some(StartingItems::from_u64(1));}
    if patch_config.game_config.random_starting_items.is_none()        {patch_config.game_config.random_starting_items        = Some(StartingItems::from_u64(0));}
    if patch_config.game_config.etank_capacity.is_none()               {patch_config.game_config.etank_capacity               = Some(100);}
    if patch_config.game_config.max_obtainable_missiles.is_none()      {patch_config.game_config.max_obtainable_missiles      = Some(999);}
    if patch_config.game_config.max_obtainable_power_bombs.is_none()   {patch_config.game_config.max_obtainable_power_bombs   = Some(8);}
    if patch_config.game_config.comment.is_none()                      {patch_config.game_config.comment                      = Some("".to_string());}
    if patch_config.game_config.main_menu_message.is_none()            {patch_config.game_config.main_menu_message            = Some("randomprime".to_string());}

    if patch_config.layout.is_none()
    {
        patch_config.layout = Some(
            LayoutWrapper::String(
                "NCiq7nTAtTnqPcap9VMQk_o8Qj6ZjbPiOdYDB5tgtwL_f01-UpYklNGnL-gTu5IeVW3IoUiflH5LqNXB3wVEER4".to_string()
            )
        );
    }

    if patch_config.game_config.game_banner.is_none()
    {
        patch_config.game_config.game_banner = Some(
            GameBanner {
                game_name: None,
                game_name_full: None,
                developer: None,
                developer_full: None,
                description: None,
            }
        );
    }

    // convert to native types used by patches.rs and return
    patch_config.parse()
}

// Copy config from b to a, skipping absent values (None)
fn merge_config(a: &mut PatchConfigPrivate, b: &PatchConfigPrivate)
{
    // TODO: prime realestate for some macros, a shame I'm too dumb to figure it out - toasterparty
    if b.input_iso.is_some()                                {a.input_iso                                = b.input_iso.clone();}
    if b.output_iso.is_some()                               {a.output_iso                               = b.output_iso.clone();}
    if b.layout.is_some()                                   {a.layout                                   = b.layout.clone();}

    if b.preferences.skip_hudmenus.is_some()                {a.preferences.skip_hudmenus                = b.preferences.skip_hudmenus;}
    if b.preferences.obfuscate_items.is_some()              {a.preferences.obfuscate_items              = b.preferences.obfuscate_items;}
    if b.preferences.artifact_hint_behavior.is_some()       {a.preferences.artifact_hint_behavior       = b.preferences.artifact_hint_behavior.clone();}
    if b.preferences.trilogy_disc_path.is_some()            {a.preferences.trilogy_disc_path            = b.preferences.trilogy_disc_path.clone();}
    if b.preferences.keep_fmvs.is_some()                    {a.preferences.keep_fmvs                    = b.preferences.keep_fmvs;}
    if b.preferences.quickplay.is_some()                    {a.preferences.quickplay                    = b.preferences.quickplay;}
    if b.preferences.quiet.is_some()                        {a.preferences.quiet                        = b.preferences.quiet;}

    if b.game_config.skip_frigate.is_some()                 {a.game_config.skip_frigate                 = b.game_config.skip_frigate;}
    if b.game_config.nonvaria_heat_damage.is_some()         {a.game_config.nonvaria_heat_damage         = b.game_config.nonvaria_heat_damage;}
    if b.game_config.staggered_suit_damage.is_some()        {a.game_config.staggered_suit_damage        = b.game_config.staggered_suit_damage;}
    if b.game_config.heat_damage_per_sec.is_some()          {a.game_config.heat_damage_per_sec          = b.game_config.heat_damage_per_sec;}
    if b.game_config.auto_enabled_elevators.is_some()       {a.game_config.auto_enabled_elevators       = b.game_config.auto_enabled_elevators;}
    if b.game_config.enable_vault_ledge_door.is_some()      {a.game_config.enable_vault_ledge_door      = b.game_config.enable_vault_ledge_door;}
    if b.game_config.starting_items.is_some()               {a.game_config.starting_items               = b.game_config.starting_items.clone();}
    if b.game_config.random_starting_items.is_some()        {a.game_config.random_starting_items        = b.game_config.random_starting_items.clone();}
    if b.game_config.etank_capacity.is_some()               {a.game_config.etank_capacity               = b.game_config.etank_capacity;}
    if b.game_config.max_obtainable_missiles.is_some()      {a.game_config.max_obtainable_missiles      = b.game_config.max_obtainable_missiles;}
    if b.game_config.max_obtainable_power_bombs.is_some()   {a.game_config.max_obtainable_power_bombs   = b.game_config.max_obtainable_power_bombs;}
    if b.game_config.game_banner.is_some()                  {a.game_config.game_banner                  = b.game_config.game_banner.clone();}
    if b.game_config.comment.is_some()                      {a.game_config.comment                      = b.game_config.comment.clone();}
    if b.game_config.main_menu_message.is_some()            {a.game_config.main_menu_message            = b.game_config.main_menu_message.clone();}
}

impl PatchConfigPrivate
{
    fn parse(&self) -> Result<PatchConfig, String>
    {
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
            let artifact_hint_behavior_string = self.preferences.artifact_hint_behavior.as_ref().unwrap().trim().to_lowercase();

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
    
        let flaahgra_music_files = if let Some(path) = self.preferences.trilogy_disc_path.as_ref() {
            Some(extract_flaahgra_music_files(&path)?)
        } else {
            None
        };

        Ok(PatchConfig {
            input_iso,
            iso_format,
            output_iso,
            layout,

            skip_hudmenus: self.preferences.skip_hudmenus.unwrap(),
            obfuscate_items: self.preferences.obfuscate_items.unwrap(),
            artifact_hint_behavior,
            flaahgra_music_files,
            keep_fmvs: self.preferences.keep_fmvs.unwrap(),
            suit_hue_rotate_angle: None,
            quiet: self.preferences.quiet.unwrap(),
            quickplay: self.preferences.quickplay.unwrap(),

            skip_frigate: self.game_config.skip_frigate.unwrap(),
            nonvaria_heat_damage: self.game_config.nonvaria_heat_damage.unwrap(),
            staggered_suit_damage: self.game_config.staggered_suit_damage.unwrap(),
            heat_damage_per_sec: self.game_config.heat_damage_per_sec.unwrap(),
            auto_enabled_elevators: self.game_config.auto_enabled_elevators.unwrap(),
            enable_vault_ledge_door: self.game_config.enable_vault_ledge_door.unwrap(),

            starting_items: self.game_config.starting_items.as_ref().unwrap().clone(),
            random_starting_items: self.game_config.random_starting_items.as_ref().unwrap().clone(),

            etank_capacity: self.game_config.etank_capacity.unwrap(),
            max_obtainable_missiles: self.game_config.max_obtainable_missiles.unwrap(),
            max_obtainable_power_bombs: self.game_config.max_obtainable_power_bombs.unwrap(),

            game_banner: self.game_config.game_banner.as_ref().unwrap().clone(),
            comment: self.game_config.comment.as_ref().unwrap().to_string(),
            main_menu_message: self.game_config.main_menu_message.as_ref().unwrap().to_string(),
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
