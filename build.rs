use std::env;
use std::path::{Path};
use std::process::Command;

use dol_linker::{read_symbol_table, link_obj_files_to_bin, link_obj_files_to_rel};

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
        .output()
        .expect("Failed to compile ppc crate");
    if !output.status.success() {
        panic!("{:#?}", output);
    }
}

fn main()
{
    let root_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let root_dir = Path::new(&root_dir);

    let ppc_dir = root_dir.join("compile_to_ppc");
    let ppc_manifest = ppc_dir.join("Cargo.toml");
    let ppc_target_dir = ppc_dir.join("target/powerpc-unknown-linux-gnu/release");

    invoke_cargo(&ppc_manifest, "rel_loader");
    invoke_cargo(&ppc_manifest, "rel_patches");

    for version in &["1.00", "1.02"] {
        let sym_table_path = format!("dol_symbol_table/{}.txt", version);
        eprintln!("{:?}", root_dir.join(&sym_table_path));
        let symbol_table = read_symbol_table(root_dir.join(sym_table_path)).unwrap();

        let bin_path = root_dir.join(format!("extra_assets/rel_loader_{}.bin", version));
        link_obj_files_to_bin(
            [ppc_target_dir.join("librel_loader.a")].iter(),
            0x80002000,
            &symbol_table,
            &bin_path,
        ).unwrap();
        println!("caragp:rerun-if-changed={}", bin_path.display());

        let rel_path = root_dir.join(format!("extra_assets/patches_{}.rel", version));
        link_obj_files_to_rel(
            [ppc_target_dir.join("librel_patches.a")].iter(),
            &symbol_table,
            &rel_path,
        ).unwrap();
        println!("caragp:rerun-if-changed={}", rel_path.display());
    }

    for entry in WalkDir::new(ppc_dir) {
        println!("caragp:rerun-if-changed={}", entry.unwrap().path().display());
    }
}
