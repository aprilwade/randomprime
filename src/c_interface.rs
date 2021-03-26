


use crate::patches;



use crate::patch_config::PatchConfig;

use std::{
    cell::Cell,
    ffi::{CStr, CString},
    panic,
    path::Path,
    os::raw::c_char,
};

use serde::{Serialize};

#[derive(Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
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

    let patch_config = PatchConfig::from_json(config_json)?;

    let pn = ProgressNotifier::new(cb_data, cb);
    patches::patch_iso(patch_config, pn)?;
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
