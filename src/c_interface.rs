
use ::{memmap, patcher, reader_writer, serde_json, structs};

use std::ffi::{CStr, CString};
use std::fs::{File, OpenOptions};
use std::panic;
use std::os::raw::c_char;

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
    layout_string: String,

    #[serde(default)]
    iso_format: patcher::IsoFormat,

    #[serde(default)]
    skip_frigate: bool,
    #[serde(default)]
    skip_hudmenus: bool,
    #[serde(default)]
    obfuscate_items: bool,

    #[serde(default)]
    keep_fmvs: bool,

    starting_items: Option<u64>,
    #[serde(default)]
    comment: String,

    banner: Option<ConfigBanner>,
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
    let input_iso = memmap::Mmap::open(&input_iso_file, memmap::Protection::Read)
            .map_err(|e| format!("Failed to open {}: {}", config.input_iso,  e))?;

    let output_iso = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&config.output_iso)
        .map_err(|e| format!("Failed to open {}: {}", config.output_iso, e))?;

    let (pickup_layout, elevator_layout, seed) = ::parse_layout(&config.layout_string)?;

    let mut config = config;
    let parsed_config = patcher::ParsedConfig {
        input_iso, output_iso,
        pickup_layout, elevator_layout, seed,

        layout_string: config.layout_string,

        iso_format: config.iso_format,
        skip_frigate: config.skip_frigate,
        skip_hudmenus: config.skip_hudmenus,
        keep_fmvs: config.keep_fmvs,
        obfuscate_items: config.obfuscate_items,
        quiet: false,

        starting_items: config.starting_items,
        comment: config.comment,

        bnr_game_name: config.banner.as_mut().and_then(|b| b.game_name.take()),
        bnr_developer: config.banner.as_mut().and_then(|b| b.developer.take()),

        bnr_game_name_full: config.banner.as_mut().and_then(|b| b.game_name_full.take()),
        bnr_developer_full: config.banner.as_mut().and_then(|b| b.developer_full.take()),
        bnr_description: config.banner.as_mut().and_then(|b| b.description.take()),
    };

    let pn = ProgressNotifier::new(cb_data, cb);
    patcher::patch_iso(parsed_config, pn)?;
    Ok(())
}

#[no_mangle]
pub extern fn randomprime_patch_iso(config_json: *const c_char , cb_data: *const (),
                                    cb: extern fn(*const (), *const c_char))
{
    let r = panic::catch_unwind(|| inner(config_json, cb_data, cb))
        .map_err(|e| {
            let msg = if let Some(e) = e.downcast_ref::<&'static str>() {
                e.to_string()
            } else if let Some(e) = e.downcast_ref::<String>() {
                e.clone()
            } else {
                format!("{:?}", e)
            };
            format!("parsing input iso failed: {}", msg)
        })
        .and_then(|i| i);

    match r {
        Ok(()) => cb(cb_data, CbMessage::success_json().as_ptr()),
        Err(s) => {
            cb(cb_data, CbMessage::error_json(&s).as_ptr())
        },
    };
}
