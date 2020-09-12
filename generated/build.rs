use std::env;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

use dol_linker::{read_symbol_table, link_obj_files_to_bin, link_obj_files_to_rel};

use flate2::read::DeflateEncoder;
use walkdir::WalkDir;


fn invoke_cargo(ppc_manifest: &Path, package: &str)
{
    let output = Command::new("rustup")
        .arg("run")
        .arg("nightly")
        .arg("cargo")
        .arg("rustc")
        .arg("--manifest-path")
        .arg(ppc_manifest)
        .arg("-p")
        .arg(package)
        .arg("--target")
        .arg("powerpc-unknown-linux-gnu")
        .arg("--release")
        .arg("--")
        .arg("-C")
        .arg("relocation-model=static")
        .arg("-C")
        .arg("target-cpu=750")
        .arg("-C")
        .arg("inline-threshold=225")
        .output()
        .expect("Failed to compile ppc crate");
    if !output.status.success() {
        panic!("{:#?}", output);
    }
}

fn build_rels(root_dir: &Path, out_dir: &Path)
{
    let ppc_dir = root_dir.join("..").join("compile_to_ppc");
    let ppc_manifest = ppc_dir.join("Cargo.toml");
    let ppc_target_dir = ppc_dir.join("target/powerpc-unknown-linux-gnu/release");

    invoke_cargo(&ppc_manifest, "rel_loader");
    invoke_cargo(&ppc_manifest, "rel_patches");

    for version in &["1.00", "1.02", "pal"] {
        let sym_table_path = format!("src/dol_symbol_table/{}.txt", version);
        eprintln!("{:?}", root_dir.join(&sym_table_path));
        let mut symbol_table = read_symbol_table(root_dir.join(sym_table_path)).unwrap();

        let bin_path = out_dir.join(format!("rel_loader_{}.bin", version));
        let symbols_map = link_obj_files_to_bin(
            [ppc_target_dir.join("librel_loader.a"), ppc_dir.join("bcmp.o")].iter(),
            0x80002000,
            &symbol_table,
            &bin_path,
        ).unwrap();
        let map_path = bin_path.with_extension("bin.map");
        {
            let mut map_file = File::create(map_path).unwrap();
            for (sym_name, addr) in &symbols_map {
                writeln!(map_file, "0x{:08x} \"{}\"", addr, sym_name).unwrap();
            }
        }

        for (sym_name, addr) in symbols_map {
            symbol_table.entry(sym_name)
                .or_insert(addr);
        }

        let rel_path = out_dir.join(format!("patches_{}.rel", version));
        link_obj_files_to_rel(
            [ppc_target_dir.join("librel_patches.a"), ppc_dir.join("bcmp.o")].iter(),
            &symbol_table,
            &rel_path,
        ).unwrap();
    }

    let walkdir = WalkDir::new(ppc_dir)
        .into_iter()
        .filter_entry(|entry|
            entry.depth() == 0 && entry.file_name().to_str().unwrap_or("") == "target"
        );
    for entry in walkdir {
        println!("cargo:rerun-if-changed={}", entry.unwrap().path().display());
    }
}

fn build_web_tracker_assets(root_dir: &Path, out_dir: &Path)
{
    let assets_dir = root_dir.join("../extra_assets/web_tracker/");
    let iter = WalkDir::new(&assets_dir)
        .min_depth(1)
        .into_iter()
        // Filter out dot files
        .filter_entry(|entry| entry
            .path()
            .file_name()
            .map(|n| !n.to_string_lossy().starts_with('.'))
            .unwrap_or(true));

    let output_file_path = out_dir.join("web_tracker_generated.rs");
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

fn main()
{
    let root_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let root_dir = Path::new(&root_dir);

    let out_dir = env::var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir);

    build_rels(&root_dir, &out_dir);
    build_web_tracker_assets(&root_dir, &out_dir);
}
