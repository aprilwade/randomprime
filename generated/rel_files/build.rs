use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;

use dol_linker::{read_symbol_table, link_obj_files_to_bin, link_obj_files_to_rel};

use walkdir::WalkDir;


fn invoke_cargo(ppc_manifest: &Path, package: &str)
{
    let output = Command::new("rustup")
        .arg("run")
        .arg("stable")
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
        .env("RUSTC_BOOTSTRAP", "1")
        .env("CARGO_TARGET_DIR", "../../compile_to_ppc/target")
        .output()
        .expect("Failed to compile ppc crate");
    if !output.status.success() {
        panic!("{:#?}", output);
    }
}

fn main()
{
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir);

    let root_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let root_dir = Path::new(&root_dir);

    let ppc_dir = root_dir.join("..").join("..").join("compile_to_ppc");
    let ppc_manifest = ppc_dir.join("Cargo.toml");
    let target_dir = ppc_dir
        .join("..")
        .join("compile_to_ppc")
        .join("target")
        .join("powerpc-unknown-linux-gnu")
        .join("release");

    let symbol_table_dir = root_dir
        .join("..")
        .join("dol_symbol_table");

    invoke_cargo(&ppc_manifest, "rel_loader");
    invoke_cargo(&ppc_manifest, "rel_patches");

    for version in &["1.00", "1.02", "pal"] {
        let sym_table_path = symbol_table_dir.join(format!("{}.txt", version));
        eprintln!("{:?}", root_dir.join("..").join(&sym_table_path));
        let mut symbol_table = read_symbol_table(root_dir.join(sym_table_path)).unwrap();

        let bin_path = out_dir.join(format!("rel_loader_{}.bin", version));
        let symbols_map = link_obj_files_to_bin(
            [target_dir.join("librel_loader.a")].iter(),
            0x80002000,
            &symbol_table,
            &bin_path,
        ).unwrap();
        let map_path = bin_path.with_extension("bin.map");
        {
            let mut map_file = File::create(map_path).unwrap();
            for (sym_name, addr) in &symbols_map {
                writeln!(map_file, "0x{:08x} {}", addr, sym_name).unwrap();
            }
        }

        for (sym_name, addr) in symbols_map {
            symbol_table.entry(sym_name)
                .or_insert(addr);
        }

        let rel_path = out_dir.join(format!("patches_{}.rel", version));
        link_obj_files_to_rel(
            [target_dir.join("librel_patches.a")].iter(),
            &symbol_table,
            &rel_path,
        ).unwrap();
    }

    let walkdir = WalkDir::new(ppc_dir)
        .into_iter()
        .filter_entry(|entry| {
            let name = entry.file_name().to_str().unwrap_or("");
            !name.starts_with(".") && name != "target"
        });
    for entry in walkdir {
        let entry = entry.unwrap();
        println!("cargo:rerun-if-changed={}", entry.path().display());
    }
}
