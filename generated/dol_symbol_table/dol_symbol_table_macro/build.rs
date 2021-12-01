use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

fn main()
{
    let output_path = Path::new(&env::var("OUT_DIR").unwrap()).join("codegen.rs");
    let mut output_file = BufWriter::new(File::create(&output_path).unwrap());

    const GAME_VERSIONS: &[(&str, &str)] = &[
        ("1.00.txt", "MP1_100_SYMBOL_TABLE"),
        ("1.01.txt", "MP1_101_SYMBOL_TABLE"),
        ("1.02.txt", "MP1_102_SYMBOL_TABLE"),
        ("pal.txt", "MP1_PAL_SYMBOL_TABLE"),
        ("kor.txt", "MP1_KOR_SYMBOL_TABLE"),
        ("jap.txt", "MP1_JAP_SYMBOL_TABLE"),
        // ("trilogy_ntsc_j.txt", "MP1_TRILOGY_NTSC_J_SYMBOL_TABLE"),
        // ("trilogy_ntsc_u.txt", "MP1_TRILOGY_NTSC_U_SYMBOL_TABLE"),
        // ("trilogy_pal.txt", "MP1_TRILOGY_PAL_SYMBOL_TABLE"),
    ];

    for (file_name, table_name) in GAME_VERSIONS {
        let symbol_path = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("..")
            .join(file_name);
        let symbol_file = BufReader::new(File::open(&symbol_path).unwrap());

        write!(
            &mut output_file,
            "static {}: phf::Map<&'static str, u32> = ",
            table_name,
        ).unwrap();
        let symbols = symbol_file.lines()
            .filter_map(|line| {
                let line = line.unwrap();
                if line.len() == 0 {
                    None
                } else {
                    assert_eq!(&line[..2], "0x");
                    let addr = &line[2..10];
                    assert_eq!(&line[10..11], " ");
                    let sym_name = &line[11..];
                    Some((sym_name.to_string(), u32::from_str_radix(addr, 16).unwrap()))
                }
            })
            .collect::<Vec<_>>();
        let mut map_generator = phf_codegen::Map::new();
        for (sym_name, sym_addr) in &symbols {
            map_generator.entry(&sym_name[..], &format!("0x{:X}", sym_addr));
        }
        write!(&mut output_file, "{}", map_generator.build()).unwrap();
        write!(&mut output_file, ";\n").unwrap();
    }
}
