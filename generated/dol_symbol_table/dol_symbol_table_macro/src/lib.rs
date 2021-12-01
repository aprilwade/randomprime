include!(concat!(env!("OUT_DIR"), "/codegen.rs"));

extern crate proc_macro;
use proc_macro::TokenStream;
use syn::{LitStr, parse_macro_input};
use quote::quote;

macro_rules! build_lookup_macros {
    ($(($macro_name:ident, $table_name:ident, $version_name:expr),)*) => {
        $(
        #[proc_macro]
        pub fn $macro_name(item: TokenStream) -> TokenStream
        {
            let sym_name = parse_macro_input!(item as LitStr);
            if let Some(addr) = $table_name.get(&sym_name.value()[..]) {
                (quote! { Some(#addr) }).into()
            } else {
                // TODO: Renable this after
                // sym_name.span()
                //     .unwrap()
                //     .warning(format!("No address for version {}", $version_name));
                (quote! { None }).into()
            }
        }
        )*
    };
}

build_lookup_macros! {
    (mp1_100_symbol, MP1_100_SYMBOL_TABLE, "NTSC 1.00"),
    (mp1_101_symbol, MP1_101_SYMBOL_TABLE, "NTSC 1.01"),
    (mp1_102_symbol, MP1_102_SYMBOL_TABLE, "NTSC 1.02"),
    (mp1_pal_symbol, MP1_PAL_SYMBOL_TABLE, "PAL"),
    (mp1_kor_symbol, MP1_KOR_SYMBOL_TABLE, "KOR"),
    (mp1_jap_symbol, MP1_JAP_SYMBOL_TABLE, "JAP"),
    // (mp1_trilogy_ntsc_j_symbol, MP1_TRILOGY_NTSC_J_SYMBOL_TABLE, "Trilogy NTSC-J"),
    // (mp1_trilogy_ntsc_u_symbol, MP1_TRILOGY_NTSC_U_SYMBOL_TABLE, "Trilogy NTSC-U"),
    // (mp1_trilogy_pal_symbol, MP1_TRILOGY_PAL_SYMBOL_TABLE, "Trilogy PAL"),
}
