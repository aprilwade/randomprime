

use enum_map::EnumMap;
use serde::{Serialize, Deserialize};

use crate::elevators::{Elevator, SpawnRoom};
use crate::patches;
use crate::starting_items::StartingItems;

use std::{
    cell::Cell,
    ffi::{CStr, CString},
    fs::{File, OpenOptions},
    panic,
    path::Path,
    os::raw::c_char,
};

#[derive(Deserialize)]
struct ConfigBanner
{
    game_name: Option<String>,
    developer: Option<String>,

    game_name_full: Option<String>,
    developer_full: Option<String>,
    description: Option<String>,
}

#[derive(Deserialize)]
struct Config
{
    input_iso: String,
    output_iso: String,
    #[serde(alias = "layout")]
    layout_string: String,

    #[serde(default)]
    iso_format: patches::IsoFormat,

    #[serde(default)]
    skip_frigate: bool,
    #[serde(default)]
    skip_hudmenus: bool,
    etank_capacity: Option<u32>,
    #[serde(default)]
    nonvaria_heat_damage: bool,
    heat_damage_per_sec: Option<f32>,
    #[serde(default)]
    staggered_suit_damage: bool,
    max_obtainable_missiles: Option<u32>,
    max_obtainable_power_bombs: Option<u32>,
    #[serde(default)]
    obfuscate_items: bool,
    #[serde(default)]
    auto_enabled_elevators: bool,

    #[serde(default)]
    skip_impact_crater: bool,
    #[serde(default)]
    enable_vault_ledge_door: bool,
    #[serde(default)]
    artifact_hint_behavior: patches::ArtifactHintBehavior,

    #[serde(default)]
    trilogy_disc_path: Option<String>,

    #[serde(default)]
    keep_fmvs: bool,

    starting_items: Option<StartingItemsWrapper>,
    random_starting_items: Option<StartingItemsWrapper>,
    #[serde(default)]
    comment: String,
    #[serde(default)]
    main_menu_message: String,

    banner: Option<ConfigBanner>,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum StartingItemsWrapper
{
    Int(u64),
    Struct(StartingItems),
}

impl Into<StartingItems> for StartingItemsWrapper
{
    fn into(self) -> StartingItems
    {
        match self {
            StartingItemsWrapper::Struct(s) => s,
            StartingItemsWrapper::Int(i) => StartingItems::from_u64(i),
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum CbMessage<'a>
{
    Success,
    Error {
        msg: &'a str,
    },
    Progress {
        percent: f64,
        msg: &'a str,
    },
}

impl<'a> CbMessage<'a>
{
    fn success_json() -> CString
    {
        CString::new(serde_json::to_string(&CbMessage::Success).unwrap()).unwrap()
    }

    fn error_json(msg: &str) -> CString
    {
        let msg = CbMessage::fix_msg(msg);
        let cbmsg = CbMessage::Error { msg };
        CString::new(serde_json::to_string(&cbmsg).unwrap()).unwrap()
    }

    fn progress_json(percent: f64, msg: &str) -> CString
    {
        let msg = CbMessage::fix_msg(msg);
        let cbmsg = CbMessage::Progress { percent, msg };
        CString::new(serde_json::to_string(&cbmsg).unwrap()).unwrap()
    }

    /// Remove all of the bytes after the first null byte
    fn fix_msg(msg: &str) -> &str
    {
        if let Some(pos) = msg.bytes().position(|i| i == b'\0') {
            &msg[..pos]
        } else {
            msg
        }
    }
}


struct ProgressNotifier
{
    total_size: usize,
    bytes_so_far: usize,
    cb_data: *const (),
    cb: extern fn(*const (), *const c_char)
}

impl ProgressNotifier
{
    fn new(cb_data: *const (), cb: extern fn(*const (), *const c_char))
        -> ProgressNotifier
    {
        ProgressNotifier {
            total_size: 0,
            bytes_so_far: 0,
            cb, cb_data
        }
    }
}

impl structs::ProgressNotifier for ProgressNotifier
{
    fn notify_total_bytes(&mut self, total_size: usize)
    {
        self.total_size = total_size
    }

    fn notify_writing_file(&mut self, file_name: &reader_writer::CStr, file_bytes: usize)
    {
        let percent = self.bytes_so_far as f64 / self.total_size as f64 * 100.;
        let msg = format!("Writing file {:?}", file_name);
        (self.cb)(self.cb_data, CbMessage::progress_json(percent, &msg).as_ptr());
        self.bytes_so_far += file_bytes;
    }

    fn notify_writing_header(&mut self)
    {
        let percent = self.bytes_so_far as f64 / self.total_size as f64 * 100.;
        (self.cb)(self.cb_data, CbMessage::progress_json(percent, "Writing ISO header").as_ptr());
    }

    fn notify_flushing_to_disk(&mut self)
    {
        (self.cb)(
            self.cb_data,
            CbMessage::progress_json(100., "Flushing written data to the disk").as_ptr(),
        );
    }
}

fn inner(config_json: *const c_char, cb_data: *const (), cb: extern fn(*const (), *const c_char))
    -> Result<(), String>
{
    let config_json = unsafe { CStr::from_ptr(config_json) }.to_str()
        .map_err(|e| format!("JSON parse failed: {}", e))?;

    let config: Config = serde_json::from_str(&config_json)
        .map_err(|e| format!("JSON parse failed: {}", e))?;

    let input_iso_file = File::open(config.input_iso.trim())
                .map_err(|e| format!("Failed to open {}: {}", config.input_iso, e))?;
    let input_iso = unsafe { memmap::Mmap::map(&input_iso_file) }
            .map_err(|e| format!("Failed to open {}: {}", config.input_iso,  e))?;

    let output_iso = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&config.output_iso)
        .map_err(|e| format!("Failed to open {}: {}", config.output_iso, e))?;

    let (pickup_layout, elevator_nums, seed) = crate::parse_layout(&config.layout_string)?;

    let starting_location = SpawnRoom::from_u32(*elevator_nums.last().unwrap() as u32).unwrap();
    let mut elevator_layout = EnumMap::<Elevator, SpawnRoom>::new();
    elevator_layout.extend(elevator_nums[..(elevator_nums.len() - 1)].iter()
        .zip(Elevator::iter())
        .map(|(i, elv)| (elv, SpawnRoom::from_u32(*i as u32).unwrap()))
    );

    let flaahgra_music_files = if let Some(path) = &config.trilogy_disc_path {
        Some(crate::extract_flaahgra_music_files(&path)?)
    } else {
        None
    };

    let mut config = config;

    let parsed_config = patches::ParsedConfig {
        input_iso, output_iso,

        pickup_layout,
        elevator_layout,
        starting_location,
        seed,

        layout_string: config.layout_string,

        iso_format: config.iso_format,
        skip_frigate: config.skip_frigate,
        skip_hudmenus: config.skip_hudmenus,
        etank_capacity: config.etank_capacity.unwrap_or(100),
        nonvaria_heat_damage: config.nonvaria_heat_damage,
        heat_damage_per_sec: config.heat_damage_per_sec.unwrap_or(10.0),
        staggered_suit_damage: config.staggered_suit_damage,
        max_obtainable_missiles: config.max_obtainable_missiles.unwrap_or(250),
        max_obtainable_power_bombs: config.max_obtainable_power_bombs.unwrap_or(8),
        keep_fmvs: config.keep_fmvs,
        obfuscate_items: config.obfuscate_items,
        auto_enabled_elevators: config.auto_enabled_elevators,
        quiet: false,

        skip_impact_crater: config.skip_impact_crater,
        enable_vault_ledge_door: config.enable_vault_ledge_door,
        artifact_hint_behavior: config.artifact_hint_behavior,

        flaahgra_music_files,

        starting_items: config.starting_items.map(|i| i.into()).unwrap_or_default(),
        random_starting_items: config.random_starting_items.map(|i| i.into()).unwrap_or(StartingItems::from_u64(0)),
        comment: config.comment,
        main_menu_message: config.main_menu_message,

        quickplay: false,

        bnr_game_name: config.banner.as_mut().and_then(|b| b.game_name.take()),
        bnr_developer: config.banner.as_mut().and_then(|b| b.developer.take()),

        bnr_game_name_full: config.banner.as_mut().and_then(|b| b.game_name_full.take()),
        bnr_developer_full: config.banner.as_mut().and_then(|b| b.developer_full.take()),
        bnr_description: config.banner.as_mut().and_then(|b| b.description.take()),
    };

    let pn = ProgressNotifier::new(cb_data, cb);
    patches::patch_iso(parsed_config, pn)?;
    Ok(())
}

#[no_mangle]
pub extern fn randomprime_patch_iso(config_json: *const c_char , cb_data: *const (),
                                    cb: extern fn(*const (), *const c_char))
{
    thread_local! {
        static PANIC_DETAILS: Cell<Option<(String, u32)>> = Cell::new(None);
    }
    panic::set_hook(Box::new(|pinfo| {
        PANIC_DETAILS.with(|pd| {
            pd.set(pinfo.location().map(|l| (l.file().to_owned(), l.line())));
        });
    }));
    let r = panic::catch_unwind(|| inner(config_json, cb_data, cb))
        .map_err(|e| {
            let msg = if let Some(e) = e.downcast_ref::<&'static str>() {
                e.to_string()
            } else if let Some(e) = e.downcast_ref::<String>() {
                e.clone()
            } else {
                format!("{:?}", e)
            };

            if let Some(pd) = PANIC_DETAILS.with(|pd| pd.replace(None)) {
                let path = Path::new(&pd.0);
                let mut comp = path.components();
                let found = path.components()
                    .skip(1)
                    .zip(&mut comp)
                    .find(|(c, _)| c.as_os_str() == "randomprime")
                    .is_some();
                // If possible, include the section of the path starting with the directory named
                // "randomprime". If no such directoy exists, just use the file name.
                let shortened_path = if found {
                    comp.as_path().as_os_str()
                } else {
                    path.file_name().unwrap_or("".as_ref())
                };
                format!("{} at {}:{}", msg, shortened_path.to_string_lossy(), pd.1)
            } else {
                msg
            }
        })
        .and_then(|i| i);

    match r {
        Ok(()) => cb(cb_data, CbMessage::success_json().as_ptr()),
        Err(msg) => cb(cb_data, CbMessage::error_json(&msg).as_ptr()),
    };
}
