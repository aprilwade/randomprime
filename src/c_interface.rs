
use ::{memmap, patcher, reader_writer, serde_json, structs};

use std::ffi::{CStr, CString};
use std::fs::{File, OpenOptions};
use std::panic;
use std::os::raw::c_char;

#[derive(Deserialize)]
struct Config
{
    input_iso: String,
    output_iso: String,
    layout_string: String,

    #[serde(default)]
    skip_frigate: bool,
    #[serde(default)]
    skip_hudmenus: bool,

    #[serde(default)]
    keep_fmvs: bool,

    starting_items: Option<u64>,
    #[serde(default)]
    comment: String,
}

#[repr(u32)]
pub enum MessageType
{
    Success = 0,
    Error = 1,
    Progress = 2,
}

struct ProgressNotifier
{
    total_size: usize,
    bytes_so_far: usize,
    cb_data: *const (),
    cb: extern fn(*const (), MessageType, *const c_char)
}

impl ProgressNotifier
{
    fn new(cb_data: *const (), cb: extern fn(*const (), MessageType, *const c_char))
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
        (self.cb)(
            self.cb_data,
            MessageType::Progress,
            CString::new(format!("{:01.0}% -- Writing file {:?}", percent, file_name)).unwrap().as_ptr()
        );
        self.bytes_so_far += file_bytes;
    }

    fn notify_writing_header(&mut self)
    {
        let percent = self.bytes_so_far as f64 / self.total_size as f64 * 100.;
        (self.cb)(
            self.cb_data,
            MessageType::Progress,
            CString::new(format!("{:02.0}% -- Writing ISO header", percent)).unwrap().as_ptr()
        );
    }

    fn notify_flushing_to_disk(&mut self)
    {
        (self.cb)(
            self.cb_data,
            MessageType::Progress,
            CString::new(format!("Flushing written data to the disk...")).unwrap().as_ptr()
        );
    }
}

fn inner(config_json: *const c_char, cb_data: *const (), cb: extern fn(*const (), MessageType, *const c_char))
    -> Result<(), String>
{
    let config_json = unsafe { CStr::from_ptr(config_json) }.to_str().map_err(|e| e.to_string())?;

    let config: Config = serde_json::from_str(&config_json).map_err(|e| e.to_string())?;

    let input_iso_file = File::open(config.input_iso.trim())
                .map_err(|e| format!("Failed to open {}: {}", config.input_iso, e))?;
    let input_iso = memmap::Mmap::open(&input_iso_file, memmap::Protection::Read)
            .map_err(|e| format!("Failed to open {}: {}", config.input_iso,  e))?;

    let output_iso = OpenOptions::new()
        .write(true)
        .create(true)
        .open(&config.output_iso)
        .map_err(|e| format!("Failed to open {}: {}", config.output_iso, e))?;
    output_iso.set_len(structs::GC_DISC_LENGTH as u64)
        .map_err(|e| format!("Failed to open {}: {}", config.output_iso, e))?;

    let (pickup_layout, elevator_layout, seed) = ::parse_layout(&config.layout_string)?;

    let parsed_config = patcher::ParsedConfig {
        input_iso, output_iso,
        pickup_layout, elevator_layout, seed,

        layout_string: config.layout_string,

        skip_frigate: config.skip_frigate,
        skip_hudmenus: config.skip_hudmenus,
        keep_fmvs: config.keep_fmvs,
        quiet: false,

        starting_items: config.starting_items,
        comment: config.comment,

    };

    let pn = ProgressNotifier::new(cb_data, cb);
    patcher::patch_iso(parsed_config, pn)?;
    Ok(())
}

#[no_mangle]
pub extern fn randomprime_patch_iso(config_json: *const c_char , cb_data: *const (),
                                    cb: extern fn(*const (), MessageType, *const c_char))
{
    let r = panic::catch_unwind(|| inner(config_json, cb_data, cb))
        .map_err(|e| {
            if let Some(e) = e.downcast_ref::<&'static str>() {
                e.to_string()
            } else if let Some(e) = e.downcast_ref::<String>() {
                e.clone()
            } else {
                format!("{:?}", e)
            }
        })
        .and_then(|i| i);

    match r {
        Ok(()) => cb(cb_data, MessageType::Success, &[0i8] as *const c_char),
        Err(s) => {
            let msg = CString::new(s)
                .unwrap_or_else(|_| CString::new("Unknown error").unwrap());
            cb(cb_data, MessageType::Error, msg.as_ptr())
        },
    };
}
