use std::env;
use std::path::Path;
use std::io::{self, Write};
use std::fs::File;

use flate2::read::DeflateEncoder;
use walkdir::WalkDir;

fn main()
{
    let root_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let root_dir = Path::new(&root_dir);

    let assets_dir = root_dir.join("assets/");
    let iter = WalkDir::new(&assets_dir)
        .min_depth(1)
        .into_iter()
        // Filter out dot files
        .filter_entry(|entry| entry
            .path()
            .file_name()
            .map(|n| !n.to_string_lossy().starts_with('.'))
            .unwrap_or(true));

    let out_dir = env::var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir);
    let output_file_path = out_dir.join("generated.rs");
    let mut output_file = File::create(output_file_path).unwrap();

    writeln!(&mut output_file, "pub static ASSETS: &'static [File] = &[").unwrap();
    for entry in iter {
        let entry = entry.unwrap();
        println!("cargo:rerun-if-changed={}", entry.path().display());
        if entry.file_type().is_dir() {
            continue
        }

        let len = entry.metadata().unwrap().len();

        let stripped = entry.path().strip_prefix(&assets_dir).unwrap();
        writeln!(&mut output_file, "    File {{").unwrap();
        writeln!(&mut output_file, "        name: {:?},", stripped).unwrap();
        writeln!(&mut output_file, "        decompressed_size: {},", len).unwrap();


        let mut f = File::open(entry.path()).unwrap();
        let mut encoder = DeflateEncoder::new(&mut f, flate2::Compression::best());
        let mut compressed_bytes = vec![];
        io::copy(&mut encoder, &mut compressed_bytes).unwrap();

        writeln!(&mut output_file, "        compressed_bytes: &[").unwrap();
        for chunk in compressed_bytes.chunks(12) {
            let line = format!("{:?}", chunk);
            writeln!(&mut output_file, "            {},", &line[1..line.len() - 1]).unwrap();

        }
        writeln!(&mut output_file, "        ],").unwrap();
        writeln!(&mut output_file, "    }},").unwrap();

    }
    writeln!(&mut output_file, "];").unwrap();
}
