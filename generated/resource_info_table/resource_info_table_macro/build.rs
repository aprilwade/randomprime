use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

fn main()
{
    let output_path = Path::new(&env::var("OUT_DIR").unwrap()).join("codegen.rs");
    let mut output_file = BufWriter::new(File::create(&output_path).unwrap());

    let resources_path = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("..")
        .join("resource_info.txt");
    let resources_file = BufReader::new(File::open(&resources_path).unwrap());

    write!(&mut output_file, "static RESOURCES: phf::Map<&'static str, &str> = ").unwrap();

    let mut resources: Vec<(String, String)> = vec![];
    for line in resources_file.lines() {
        let line = line.unwrap();
        if line.len() == 0 {
            continue;
        }
        let mut parts = line.split('"');
        assert_eq!(parts.next(), Some(""));
        let long_name = parts.next().unwrap();
        let res_id = &parts.next().unwrap()[2..12];
        let res_type = parts.next().unwrap();
        let mut pak_names = parts
            .filter(|s| !(s.contains(',') || s.contains('[') || s.contains(']') || s.is_empty()))
            .collect::<Vec<_>>();

        let last_pak = pak_names.last().unwrap();
        assert!(last_pak.len() > 3, "{:?} {:?}", long_name, pak_names);
        let short_name = if last_pak[last_pak.len() - 4..].to_lowercase() != ".pak" {
            pak_names.pop()
        } else {
            None
        };

        let pak_names_formatted = pak_names.iter()
            .map(|name| format!("b{:?}", name))
            .collect::<Vec<_>>()
            .join(", ");

        let resource_data = format!("
            r#\"resource_info_table::ResourceInfo {{
                long_name: {:?},
                short_name: {:?},
                res_id: {},
                fourcc: reader_writer::FourCC::from_bytes(b\"{}\"),
                paks: &[{}],
            }}\"#", long_name, short_name, res_id, res_type, pak_names_formatted);
        if let Some(short_name) = short_name {
            resources.push((
                short_name.to_string(),
                resource_data.clone()
            ));
        }
        resources.push((long_name.to_string(), resource_data));
    }
    let mut map_generator = phf_codegen::Map::new();
    for (resource_name, resource_data) in &resources {
        map_generator.entry(&resource_name[..], resource_data);
    }
    write!(&mut output_file, "{}", map_generator.build()).unwrap();
    write!(&mut output_file, ";\n").unwrap();
}
