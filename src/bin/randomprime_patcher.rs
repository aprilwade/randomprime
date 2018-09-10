#[macro_use] extern crate clap;
extern crate preferences;
extern crate memmap;
extern crate randomprime;
extern crate winapi;

use clap::{Arg, App};
// XXX This is an undocumented enum
use clap::Format;
use preferences::{AppInfo, PreferencesMap, Preferences};

// pub use randomprime::*;
use randomprime::{parse_layout, patcher, pickup_meta, reader_writer, structs};

use std::borrow::Cow;
use std::collections::HashMap;
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::panic;
use std::process::Command;


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
            quiet: quiet,
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

fn interactive() -> Result<patcher::ParsedConfig, String>
{
    fn read_option<R, F>(prompt: &str, default: &str, question: &str, f: F) -> Result<R, String>
        where F: Fn(&str) -> Result<R, String>
    {
        let mut s = String::new();

        loop {
            if default.len() > 0 {
                print!("\n{} ({}): ", prompt, default);
            } else {
                print!("\n{}: ", prompt);
            }

            io::stdout().flush().map_err(|e| format!("Interactive IO error: {}", e))?;

            s.clear();
            io::stdin().read_line(&mut s)
                .map_err(|e| format!("Failed to read from stdin: {}", e))?;
            let s = s.trim();

            if s == "?" {
                println!("{}", question);
                continue;
            }

            let res = if s.len() > 0 {
                f(&s)
            } else if default.len() > 0 {
                f(default)
            } else {
                Err("A response is required".into())
            };

            match res {
                // XXX: Do I really want stderr?
                Err(s) => writeln!(io::stderr(), "{} {}", Format::Error("error:"), s).unwrap(),
                Ok(ret) => return Ok(ret),
            };

        }
    }

    const APP_INFO: AppInfo = AppInfo {
        name: "com.wayedt.randomprime",
        author: "April Wade",
    };

    let prefs_key = "mp1";
    let mut prefs = PreferencesMap::<String>::load(&APP_INFO, prefs_key).unwrap_or(HashMap::new());

    println!("Metroid Prime Randomizer ISO Patcher");
    println!("Version {}", crate_version!());
    println!("");
    println!("Interactive mode");
    println!("I need to collect some information from you before I can modify your ISO.");
    println!("If you want more information about any given option, you may enter a ?.");
    println!("The text in () is the default or last used choice, if one exists.");

    let passed_in_iso_data = if was_launched_by_windows_explorer() {
        // catch-blocks aren't stable yet...
        (|| {
            let input_iso_path = env::args().nth(1)?;
            let try_opening = (|| {
                let input_iso_file = File::open(input_iso_path.trim())
                            .map_err(|e| format!("Failed to open {}: {}", input_iso_path, e))?;
                memmap::Mmap::open(&input_iso_file, memmap::Protection::Read)
                            .map_err(|e| format!("Failed to open {}: {}", input_iso_path,  e))
                            .map(|m| (input_iso_path.to_string(), m))
            })();
            match try_opening {
                Ok(res) => Some(res),
                Err(res) => {
                    println!("Failed to open ISO file passed from Explorer: {}", res);
                    None
                },
            }
        })()
    } else {
        None
    };

    let (input_iso_path, input_iso_mmap) = if let Some(piid) = passed_in_iso_data {
        piid
    } else {
        let help_message = if cfg!(windows) {
            concat!(
                "\nThis is the location of an unmodified copy of the Metroid Prime ISO.",
                "\nIf you ran this program by double clicking on it, and the ISO file is in the",
                "\nsame folder, you can simply enter the name of the file. Otherwise, you need to",
                "\nenter an absolute path, which probably should start with a drive letter (eg C:\\)",
                "\nA shortcut to doing that is to drag and drop the ISO file onto this CMD window.",
                "\nAlternatively, if you relaunch this program by dragging and dropping your ISO",
                "\nfile onto the patcher's EXE file, this option will be handled automatically."
            )
        } else {
            "\nThis is the location of an unmodified copy of the Metroid Prime ISO."
        };
        read_option(
            "Input file name", prefs.get("input_iso").map(|x| x.as_str()).unwrap_or(""),
           help_message,
            |input_iso_path| {
                let bytes = input_iso_path.as_bytes();
                let input_iso_path = if bytes[0] == b'"' && bytes[2] == b':' && bytes[3] == b'\\'
                                        && bytes.ends_with(b"\"") {
                    Cow::Owned(input_iso_path[1..(input_iso_path.len() - 1)].to_string())
                } else {
                    Cow::Borrowed(input_iso_path)
                };
                let input_iso_file = File::open(input_iso_path.trim())
                            .map_err(|e| format!("Failed to open {}: {}", input_iso_path, e))?;
                memmap::Mmap::open(&input_iso_file, memmap::Protection::Read)
                            .map_err(|e| format!("Failed to open {}: {}", input_iso_path,  e))
                            .map(|m| (input_iso_path.to_string(), m))
        })?
    };

    let layout_help_message = if cfg!(windows) {
        concat!("\nThis is the string that describes which pickups are placed where. If you don't",
                "\nalready have one, go to https://etaylor8086.github.io/randomizer/ generate one.",
                "\nIts suggested that you copy-paste the string rather than try to re-type it. If",
                "\nyou launched the patcher from Explorer, you maybe have to right-click on the",
                "\ntitle-bar and then look under the \"edit\" menu to paste.")
    } else {
        concat!("\nThis is the string that describes which pickups are placed where. If you don't",
                "\nalready have one, go to https://etaylor8086.github.io/randomizer/ generate one.",
                "\nIts suggested that you copy-paste the string rather than try to re-type it.")
    };
    let (pickup_layout, elevator_layout, seed, layout_string) = read_option(
        "Layout descriptor", "",
        layout_help_message,
        |pickup_layout| {
            let pickup_layout = pickup_layout.trim().to_string();
            parse_layout(&pickup_layout).map(|i| (i.0, i.1, i.2, pickup_layout))
    })?;

    let match_bool = |resp: &str| match resp.trim() {
            "Y" | "y" | "Yes" | "yes" => Ok(true),
            "N" | "n" | "No"  | "no"  => Ok(false),
            n => Err(format!("Invalid response {}. Expected Yes/No.", n)),
        };
    let skip_frigate = read_option(
        "Skip the frigate level?", prefs.get("skip_frigate").map(|x| x.as_str()).unwrap_or("Yes"),
        concat!("\nIf yes, new save files will start at the Landing Site in Tallon Overworld",
                "\ninstead of the Space Pirate Frigate."),
        &match_bool
    )?;

    /* let keep_fmvs = read_option(
        "Remove attract mode?", "Yes", "If yes, the attract mode FMVs are remov",
        &match_bool
    )?;*/
    /* let skip_hudmenus = read_option(
        "Non-modal item messages?", "Yes", "",
        &match_bool)?;*/

    let (output_iso_path, out_iso) = read_option(
        "Output file name", prefs.get("output_iso").map(|x| x.as_str()).unwrap_or(""),
        concat!("\nThis is the location where the randomized ISO will be written.",
                "\nIf the file name you provide ends in .gcz or .ciso, the iso will automatically",
                "\nbe compressed in the corresponding format as it is written.",
                "\nWarning: This will overwrite the file at the given location if one exists."),
        |output_iso_path| {
            let out_iso = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(output_iso_path)
                .map_err(|e| format!("Failed to open output file: {}", e))?;
            Ok((output_iso_path.to_string(), out_iso))
    })?;

    let iso_format = if output_iso_path.ends_with(".gcz") {
        patcher::IsoFormat::Gcz
    } else if output_iso_path.ends_with(".ciso") {
        patcher::IsoFormat::Ciso
    } else {
        patcher::IsoFormat::Iso
    };
    prefs.insert("input_iso".to_string(), input_iso_path);
    prefs.insert("output_iso".to_string(), output_iso_path);
    prefs.insert("skip_frigate".to_string(), if skip_frigate { "Y" } else { "N" }.to_string());
    let _ = prefs.save(&APP_INFO, prefs_key); // Throw away any error; its fine if this fails

    Ok(patcher::ParsedConfig {
        input_iso: input_iso_mmap,
        output_iso: out_iso,
        pickup_layout, elevator_layout, seed, layout_string,

        iso_format,
        skip_hudmenus: true,
        skip_frigate,
        keep_fmvs: false,
        quiet: false,

        starting_items: None,
        comment: "".to_string(),

        bnr_game_name: None,
        bnr_developer: None,

        bnr_game_name_full: None,
        bnr_developer_full: None,
        bnr_description: None,
    })
}

fn get_config() -> Result<patcher::ParsedConfig, String>
{
    if env::args().len() <= 1 || (was_launched_by_windows_explorer() && env::args().len() <= 2) {
        interactive()
    } else {
        let matches = App::new("randomprime ISO patcher")
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
            .arg(Arg::with_name("keep attract mode")
                .long("keep-attract-mode")
                .help("Keeps the attract mode FMVs, which are removed by default"))
            .arg(Arg::with_name("quiet")
                .long("quiet")
                .help("Don't print the progress messages"))
            .arg(Arg::with_name("change starting items")
                .long("starting-items")
                .hidden(true)
                .takes_value(true)
                .validator(|s| s.parse::<u64>().map(|_| ())
                                            .map_err(|_| "Expected an integer".to_string())))
            .arg(Arg::with_name("text file comment")
                 .long("text-file-comment")
                 .hidden(true)
                 .takes_value(true))
            .get_matches();

        let input_iso_path = matches.value_of("input iso path").unwrap();
        let input_iso_file = File::open(input_iso_path)
                    .map_err(|e| format!("Failed to open input iso: {}", e))?;
        let input_iso_mmap = memmap::Mmap::open(&input_iso_file, memmap::Protection::Read)
                    .map_err(|e| format!("Failed to open input iso: {}", e))?;

        let output_iso_path = matches.value_of("output iso path").unwrap();
        let out_iso = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(output_iso_path)
            .map_err(|e| format!("Failed to open output file: {}", e))?;

        let iso_format = if output_iso_path.ends_with(".gcz") {
            patcher::IsoFormat::Gcz
        } else if output_iso_path.ends_with(".ciso") {
            patcher::IsoFormat::Ciso
        } else {
            patcher::IsoFormat::Iso
        };
        let layout_string = matches.value_of("pickup layout").unwrap().to_string();
        let (pickup_layout, elevator_layout, seed) = parse_layout(&layout_string)?;

        Ok(patcher::ParsedConfig {
            input_iso: input_iso_mmap,
            output_iso: out_iso,
            pickup_layout, elevator_layout, seed, layout_string,

            iso_format,
            skip_hudmenus: matches.is_present("skip hudmenus"),
            skip_frigate: matches.is_present("skip frigate"),
            keep_fmvs: matches.is_present("keep attract mode"),
            quiet: matches.is_present("quiet"),

            // XXX We can unwrap safely because we verified the parse earlier
            starting_items: matches.value_of("change starting items")
                                   .map(|s| s.parse::<u64>().unwrap()),

            comment: matches.value_of("text file comment").unwrap_or("").to_string(),

            bnr_game_name: None,
            bnr_developer: None,

            bnr_game_name_full: None,
            bnr_developer_full: None,
            bnr_description: None,
        })

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
    let config = get_config()?;
    let pn = ProgressNotifier::new(config.quiet);
    patcher::patch_iso(config, pn)?;
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
            let _ = writeln!(io::stderr(), "{} \
An error occurred while parsing the input ISO. \
This most likely means your ISO is corrupt. \
Please verify that your ISO matches one of the following hashes:
MD5:  eeacd0ced8e2bae491eca14f141a4b7c
SHA1: ac20c744db18fdf0339f37945e880708fd317231
", Format::Error("error:"));

            maybe_pause_at_exit();
        }));
    }

    pickup_meta::setup_pickup_meta_table();

    let _ = match main_inner() {
        Err(s) => writeln!(io::stderr(), "{} {}", Format::Error("error:"), s),
        Ok(()) => Ok(()),
    };

    maybe_pause_at_exit();
}
