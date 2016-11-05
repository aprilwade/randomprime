extern crate auto_struct_macros;

use auto_struct_macros::syntex;


use std::env;
use std::path::Path;

fn main()
{
    syntex::with_extra_stack(|| {
        let out_dir = env::var_os("OUT_DIR").unwrap();
        let mut registry = syntex::Registry::new();
        auto_struct_macros::register(&mut registry);

        let src = Path::new("src/lib.rs.in");
        let dst = Path::new(&out_dir).join("lib.rs");

        registry.expand("", &src, &dst).unwrap();
    })
}
