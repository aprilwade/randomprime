use clap::{Arg, App, SubCommand};

use dol_linker::{read_symbol_table, link_obj_files_to_bin, link_obj_files_to_rel};

fn main()
{
    let matches = App::new("dol linker")
        .subcommand(SubCommand::with_name("rel")
            .arg(Arg::with_name("symbol-map")
                .long("symbol-map")
                .short("s")
                .takes_value(true)
                .required(true))
            // .arg(Arg::with_name("prolog-func")
            //     .long("prolog-func")
            //     .short("p")
            //     .takes_value(true)
            //     .default_value("__rel_prolog"))
            // .arg(Arg::with_name("epilog-func")
            //     .long("epilog-func")
            //     .short("e")
            //     .takes_value(true)
            //     .default_value("__rel_epilog"))
            .arg(Arg::with_name("output")
                .long("output")
                .short("o")
                .takes_value(true)
                .default_value("output.rel"))
            .arg(Arg::with_name("obj-files")
                .required(true)
                .multiple(true)
                .min_values(1)))
        .subcommand(SubCommand::with_name("bin")
            .arg(Arg::with_name("symbol-map")
                .long("symbol-map")
                .short("s")
                .takes_value(true)
                .required(true))
            .arg(Arg::with_name("load-addr")
                .long("load-addr")
                .short("a")
                .takes_value(true)
                .required(true))
            .arg(Arg::with_name("output")
                .long("output")
                .short("o")
                .takes_value(true)
                .default_value("output.bin"))
            .arg(Arg::with_name("obj-files")
                .required(true)
                .multiple(true)
                .min_values(1)))
        .subcommand(SubCommand::with_name("dol")
            .arg(Arg::with_name("entrypoint-func")
                .long("entrypoint-func")
                .short("e")
                .takes_value(true))
            .arg(Arg::with_name("output")
                .long("output")
                .short("o")
                .takes_value(true)
                .default_value("output.dol"))
            .arg(Arg::with_name("obj-files")
                .required(true)
                .multiple(true)
                .min_values(1)))
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("rel") {

        let extern_sym_table_fname = matches.value_of("symbol-map").unwrap();

        let extern_sym_table = read_symbol_table(extern_sym_table_fname).unwrap();
        link_obj_files_to_rel(
            matches.values_of("obj-files").unwrap(),
            &extern_sym_table,
            matches.value_of("output").unwrap(),
        ).unwrap();
    } else if let Some(matches) = matches.subcommand_matches("bin") {

        let extern_sym_table_fname = matches.value_of("symbol-map").unwrap();
        let load_addr = matches.value_of("load-addr").unwrap();
        let load_addr = u32::from_str_radix(load_addr, 16).unwrap();

        let extern_sym_table = read_symbol_table(extern_sym_table_fname).unwrap();
        let exported_symbols = link_obj_files_to_bin(
            matches.values_of("obj-files").unwrap(),
            load_addr,
            &extern_sym_table,
            matches.value_of("output").unwrap(),
        ).unwrap();

        println!("Exported symbol addresses:");
        for (sym_name, addr) in exported_symbols {
            println!("    {}: {}", sym_name, addr);
        }
    } else {
        println!("Unimplemented subcommand");
    }
}
