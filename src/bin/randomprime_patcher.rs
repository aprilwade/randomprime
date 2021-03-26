use randomprime::{
    patches,
    reader_writer,
    structs,
    patch_config::PatchConfig,
};

use clap::Format;

use std::{
    panic,
    process::Command,
};

struct ProgressNotifier
{
    total_size: usize,
    bytes_so_far: usize,
    quiet: bool,
}

impl ProgressNotifier
{
    fn new(quiet: bool) -> ProgressNotifier
    {
        ProgressNotifier {
            total_size: 0,
            bytes_so_far: 0,
            quiet,
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
        if self.quiet {
            return;
        }
        let percent = self.bytes_so_far as f64 / self.total_size as f64 * 100.;
        println!("{:02.0}% -- Writing file {:?}", percent, file_name);
        self.bytes_so_far += file_bytes;
    }

    fn notify_writing_header(&mut self)
    {
        if self.quiet {
            return;
        }
        let percent = self.bytes_so_far as f64 / self.total_size as f64 * 100.;
        println!("{:02.0}% -- Writing ISO header", percent);
    }

    fn notify_flushing_to_disk(&mut self)
    {
        if self.quiet {
            return;
        }
        println!("Flushing written data to the disk...");
    }
}

#[cfg(windows)]
fn was_launched_by_windows_explorer() -> bool
{
    // https://stackoverflow.com/a/513574
    use winapi::um::processenv:: *;
    use winapi::um::winbase:: *;
    use winapi::um::wincon:: *;
    static mut CACHED: Option<bool> = None;
    unsafe {
        if let Some(t) = CACHED {
            return t;
        }
        let mut csbi: CONSOLE_SCREEN_BUFFER_INFO = std::mem::zeroed();
        let x = GetConsoleScreenBufferInfo(GetStdHandle(STD_OUTPUT_HANDLE), &mut csbi);
        CACHED = Some(x == 1 && csbi.dwCursorPosition.X == 0 && csbi.dwCursorPosition.Y == 0);
        CACHED.unwrap()
    }
}

#[cfg(not(windows))]
fn was_launched_by_windows_explorer() -> bool
{
    false
}

fn maybe_pause_at_exit()
{
    if was_launched_by_windows_explorer() {
        // XXX Windows only
        let _ = Command::new("cmd.exe").arg("/c").arg("pause").status();
    }
}

fn main_inner() -> Result<(), String>
{
    let patch_config = PatchConfig::from_cli_options()?;
    let pn = ProgressNotifier::new(patch_config.quiet);
    patches::patch_iso(patch_config, pn)?;
    println!("Done");
    Ok(())
}

fn main()
{
    // XXX We have to check this before we print anything; it relies on the cursor position and
    //     caches its result.
    was_launched_by_windows_explorer();

    // On non-debug builds, suppress the default panic message and print a more helpful and
    // user-friendly one
    if !cfg!(debug_assertions) {
        panic::set_hook(Box::new(|_| {
            let _ = eprintln!("{} \
An error occurred while parsing the input ISO. \
This most likely means your ISO is corrupt. \
Please verify that your ISO matches one of the following hashes:
MD5:  eeacd0ced8e2bae491eca14f141a4b7c
SHA1: ac20c744db18fdf0339f37945e880708fd317231
", Format::Error("error:"));

            maybe_pause_at_exit();
        }));
    }

    match main_inner() {
        Err(s) => eprintln!("{} {}", Format::Error("error:"), s),
        Ok(()) => (),
    };

    maybe_pause_at_exit();
}
