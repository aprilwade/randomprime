extern crate proc_macro;

use proc_macro_hack::proc_macro_hack;

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, Ident, Token, Result
};
use syn::parse::{Parse, ParseStream};

use std::env;
use std::fs::File;
use std::path::PathBuf;
use std::io::{BufRead, BufReader};

struct MacroParams
{
    file_name: String,
    _comma: Token![,],
    macro_name: Ident,
    line_ending: Option<proc_macro2::TokenStream>,
}

impl Parse for MacroParams
{
    fn parse(input: ParseStream) -> Result<Self>
    {
        Ok(MacroParams {
            file_name: input.parse::<syn::LitStr>()?.value().to_string(),
            _comma: input.parse()?,
            macro_name: input.parse()?,
            line_ending: if input.is_empty() {
                    None
                } else {
                    let _comma1: Token![,] = input.parse()?;
                    Some(input.parse()?)
                },
        })

    }
}

fn macro_file_proxy_shared(tokens: TokenStream) -> TokenStream {
    let params = parse_macro_input!(tokens as MacroParams);

    let crate_root = env::var("CARGO_MANIFEST_DIR").unwrap();
    let path = PathBuf::from(crate_root)
                    .join(params.file_name);

    let file = BufReader::new(File::open(&path)
        .expect("Failed to open file"));
    let line_iter = file.lines().enumerate()
        .map(|(i, line)| (i, line.expect(&format!("Failed to read line {}", i))))
        .map(|(i, line)| syn::parse_str::<proc_macro2::TokenStream>(&line)
             .expect(&format!("Failed to tokenize line {}", i)));

    let macro_name = &params.macro_name;
    let line_ending_iter = std::iter::repeat(&params.line_ending);
    (quote! {
        #macro_name! { #(#line_iter #line_ending_iter)* }
    }).into()
}

#[proc_macro_hack]
pub fn macro_file_proxy(tokens: TokenStream) -> TokenStream {
    macro_file_proxy_shared(tokens)
}

#[proc_macro]
pub fn macro_file_proxy_item(tokens: TokenStream) -> TokenStream {
    macro_file_proxy_shared(tokens)
}
