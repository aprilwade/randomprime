#![recursion_limit = "128"]

extern crate proc_macro;
extern crate proc_macro2;
#[macro_use] extern crate quote;
#[macro_use] extern crate syn;

use std::fmt::Display;

use quote::ToTokens;
use proc_macro::TokenStream;
use proc_macro2::Span;
use syn::{Expr, Field, Ident, Pat, ItemStruct, Type};
use syn::parse::{Error, Parse, Parser, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

fn err<A, T: Display>(span: Span, message: T) -> Result<A>
{
    Err(Error::new(span, message))
}

struct AutoStructField
{
    ident: Ident,
    ty: Type,
    kind: AutoStructFieldKind,
}

enum AutoStructFieldKind
{
    PadAlign(Expr),
    Derivable(Expr, Expr),
    IteratorDerivable(Expr, Expr),
    Expected(Expr, Expr),
    Literal(Expr),
    Simple(Expr),
}

impl AutoStructField
{
    fn from_raw(field: &Field, raw: Option<RawAutoStructAttr>, init: Option<RawAutoStructAttr>)
        -> Result<Self>
    {
        let init_was_none = init.is_none();
        let init_expr = init
            .map(|init| syn::parse2::<Expr>(init.tts))
            .unwrap_or_else(|| syn::parse_str("()"))?;
        let raw = if let Some(raw) = raw {
            raw
        } else {
            return Ok(AutoStructField {
                ident: field.ident.clone().unwrap(),
                ty: field.ty.clone(),
                kind: AutoStructFieldKind::Simple(init_expr),
            })
        };

        let kind = if raw.ident == "derive" {
            AutoStructFieldKind::Derivable(syn::parse2(raw.tts)?, init_expr)
        } else if raw.ident == "expect" {
            AutoStructFieldKind::Expected(syn::parse2(raw.tts)?, init_expr)
        } else if raw.ident == "literal" {
            if !init_was_none {
                err(raw.ident.span(), "`literal` auto_struct field cant have an initializer")?;
            }
            AutoStructFieldKind::Literal(syn::parse2(raw.tts)?)
        } else if raw.ident == "derive_from_iter" {
            AutoStructFieldKind::IteratorDerivable(syn::parse2(raw.tts)?, init_expr)
        } else if raw.ident == "pad_align" {
            if !init_was_none {
                err(raw.ident.span(), "`pad_align` auto_struct field cant have an initializer")?;
            }
            // PadAlign is a little magic; it overrides the field type
            return Ok(AutoStructField {
                ident: field.ident.clone().unwrap(),
                ty: syn::parse_str("reader_writer::PaddingBlackhole")?,
                kind: AutoStructFieldKind::PadAlign(syn::parse2(raw.tts)?),
            })

        } else {
            err(raw.ident.span(), "Unknown auto_struct field kind")?
        };

        Ok(AutoStructField {
            ident: field.ident.clone().unwrap(),
            ty: field.ty.clone(),
            kind: kind,
        })
    }

    fn has_storage(&self) -> bool
    {
        match self.kind {
            AutoStructFieldKind::PadAlign(_) => false,
            AutoStructFieldKind::Derivable(_, _) => false,
            AutoStructFieldKind::IteratorDerivable(_, _) => false,
            AutoStructFieldKind::Expected(_, _) => false,
            AutoStructFieldKind::Literal(_) => true,
            AutoStructFieldKind::Simple(_) => true,
        }
    }

    fn needs_offset(&self) -> bool
    {
        match self.kind {
            AutoStructFieldKind::PadAlign(_) => true,
            AutoStructFieldKind::Derivable(_, _) => false,
            AutoStructFieldKind::IteratorDerivable(_, _) => false,
            AutoStructFieldKind::Expected(_, _) => false,
            AutoStructFieldKind::Literal(_) => false,
            AutoStructFieldKind::Simple(_) => false,
        }
    }

    fn read_expr(&self, struct_name: &Ident) -> proc_macro2::TokenStream
    {
        match &self.kind {
            AutoStructFieldKind::PadAlign(aligned) => quote! {
                {
                    let __curr_len__ = __reader__.len();
                    __reader__.read(reader_writer::pad_bytes_count(
                        #aligned,
                        __start_len__ - __curr_len__
                    ))
                }
            },
            AutoStructFieldKind::Derivable(_, init) => quote!(__reader__.read(#init)),
            AutoStructFieldKind::IteratorDerivable(_, init) => quote!(__reader__.read(#init)),
            AutoStructFieldKind::Expected(expected, init) => {
                let ident = &self.ident;
                let ty = &self.ty;
                let file_line = quote_spanned! { self.ident.span() =>
                    file!(), line!()
                };
                quote! {
                    {
                        let __tmp__ = __reader__.read(#init);
                        let expected: #ty = #expected;

                        // reader_writer::expect!(
                        assert_eq!(
                            expected, __tmp__,
                            "While deserializing {}, {}:{}",
                            stringify!(#struct_name::#ident), #file_line
                        );
                        __tmp__
                    }
                }
            },
            AutoStructFieldKind::Literal(expr) => quote!(#expr),
            AutoStructFieldKind::Simple(init) => quote!(__reader__.read(#init)),
        }
    }

    fn write_expr(&self) -> Option<proc_macro2::TokenStream>
    {
        match &self.kind {
            AutoStructFieldKind::PadAlign(aligned) => Some(quote! {
                reader_writer::PaddingBlackhole(reader_writer::pad_bytes_count(
                    #aligned,
                    __sum__ as usize,
                ))
            }),
            AutoStructFieldKind::Derivable(expr, _) => Some(expr.into_token_stream()),
            AutoStructFieldKind::IteratorDerivable(expr, _) => Some(quote! {
                reader_writer::Dap::new(#expr)
            }),
            AutoStructFieldKind::Expected(expected, _) => Some(expected.into_token_stream()),
            AutoStructFieldKind::Literal(_) => None,
            AutoStructFieldKind::Simple(_) => {
                let ident = &self.ident;
                Some(quote!(self.#ident))
            },
        }
    }

    fn write_type(&self) -> Option<proc_macro2::TokenStream>
    {
        let ty = &self.ty;
        match &self.kind {
            AutoStructFieldKind::Literal(_) => None,
            AutoStructFieldKind::IteratorDerivable(_, _) => Some(quote! {
                reader_writer::Dap<_, <#ty as reader_writer::DerivableFromIterator>::Item>
            }),
            _ => Some(quote!(#ty)),
        }
    }
}

#[derive(Clone)]
struct RawAutoStructAttr
{
    ident: Ident,
    eq: Option<Token![=]>,
    tts: proc_macro2::TokenStream,
}

impl Parse for RawAutoStructAttr
{
    fn parse(input: ParseStream) -> Result<Self>
    {
        let ident = input.parse()?;
        if input.peek(Token![=]) {
            Ok(RawAutoStructAttr {
                ident,
                eq: input.parse()?,
                tts: input.parse()?,
            })
        } else {
            Ok(RawAutoStructAttr {
                ident,
                eq: None,
                tts: proc_macro2::TokenStream::new(),
            })
        }
    }
}

struct DeriveOptions
{
    readable: bool,
    writable: bool,
    fixed_size: bool,
}

impl Parse for DeriveOptions
{
    fn parse(input: ParseStream) -> Result<Self>
    {
        let mut options = DeriveOptions {
            readable: false,
            writable: false,
            fixed_size: false,
        };
        let idents = Punctuated::<Ident, Token![,]>::parse_terminated(input)?;
        for ident in idents {
            if ident == "Readable" {
                if options.readable {
                    err(ident.span(), format!("Duplicate '{}'", ident))?;
                }
                options.readable = true;
            } else if ident == "Writable" {
                if options.writable {
                    err(ident.span(), format!("Duplicate '{}'", ident))?;
                }
                options.writable = true;
            } else if ident == "FixedSize" {
                if options.fixed_size {
                    err(ident.span(), format!("Duplicate '{}'", ident))?;
                }
                options.fixed_size = true;
            } else {
                err(ident.span(), format!("Unknown option '{}'", ident))?;
            }
        }

        Ok(options)
    }
}

struct AutoStructDecl
{
    struct_: ItemStruct,
    fields: Vec<AutoStructField>,
    args_pat: Pat,
    args_ty: Type,
}

impl AutoStructDecl
{
    fn from_struct(decl: syn::ItemStruct) -> Result<Self>
    {
        let mut as_struct = AutoStructDecl {
            struct_: decl,
            fields: vec![],
            args_pat: syn::parse_str("()")?,
            args_ty: syn::parse_str("()")?,
        };

        let mut seen_args = false;
        let struct_span = as_struct.struct_.span();

        {
            let fields = match &mut as_struct.struct_.fields {
                syn::Fields::Named(fields) => fields,
                _ => err(struct_span, "auto_struct requires a { } structs")?,
            };

            let mut old_fields = Punctuated::new();
            std::mem::swap(&mut fields.named, &mut old_fields);

            for mut field in old_fields {

                let mut as_attrs = vec![];
                for attr in field.attrs.iter() {
                    if attr.path.is_ident("auto_struct") {
                        let parser = |input: ParseStream| {
                            let content;
                            let _paren = parenthesized!(content in input);
                            Punctuated::<RawAutoStructAttr, Token![,]>::parse_terminated(&content)
                        };
                        let attrs = parser.parse2(attr.tokens.clone())?;
                        if attrs.len() == 0 {
                            err(attr.span(), "Empty auto_struct attribute")?;
                        }
                        as_attrs.extend(attrs.into_iter());
                    }
                }

                let mut kind_attr = None;
                let mut init_attr = None;
                for raw_attr in as_attrs {
                    if raw_attr.ident == "init" {
                        if init_attr.is_some() {
                            err(raw_attr.ident.span(),
                                "Each field may have only 1 auto_struct(init) attribute"
                            )?;
                        } else {
                            init_attr = Some(raw_attr);
                        }
                    } else {
                        if kind_attr.is_some() {
                            err(raw_attr.ident.span(),
                                "Each field may have only 1 non init auto_struct attribute"
                            )?;
                        } else {
                            kind_attr = Some(raw_attr);
                        }
                    }
                }

                if kind_attr.as_ref().map(|raw_attr| raw_attr.ident == "args").unwrap_or(false) {
                    let kind_attr = kind_attr.unwrap();
                    if let Some(init_attr) = &init_attr {
                        err(init_attr.ident.span(), "auto_struct args may not have an initializer")?
                    }
                    if seen_args {
                        err(kind_attr.ident.span(), "duplicate auto_struct args decl")?
                    }
                    seen_args = true;
                    // TODO: The error span here isn't correct?
                    as_struct.args_pat = if kind_attr.eq.is_some() {
                        syn::parse2(kind_attr.tts)?
                    } else {
                        syn::parse2((&field.ident).into_token_stream())?
                    };
                    as_struct.args_ty = field.ty;
                    continue
                }

                let as_field = AutoStructField::from_raw(&field, kind_attr, init_attr)?;

                if as_field.has_storage() {
                    field.attrs.retain(|attr| !attr.path.is_ident("auto_struct"));
                    fields.named.push(field);
                }

                as_struct.fields.push(as_field);
            }
        }

        Ok(as_struct)
    }

    fn readable_impl_tokens(&self, fixed_size: bool) -> proc_macro2::TokenStream
    {
        // If the struct contains a type parameter named R, use that for our Reader argument to
        // the Readable trait. Otherwise, we need to create a new generic parameter (still
        // named R) just for the impl.
        let mut generics = self.struct_.generics.clone();
        let (_, type_gens, _) = self.struct_.generics.split_for_impl();
        let reader_lifetime = {
            let reader_lifetime_arg = self.struct_.generics.lifetimes()
                .find(|ld| ld.lifetime.ident == "r");
            if let Some(arg) = reader_lifetime_arg {
                arg.clone()
            } else {
                generics.params.push(syn::GenericParam::Lifetime(syn::parse_str("'r").unwrap()));
                syn::parse_str("'r").unwrap()
            }
        };
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        let name = &self.struct_.ident;

        let size_fn = if fixed_size {
            let types = self.fields.iter()
                .filter(|field| field.write_expr().is_some())
                .map(|field| &field.ty);

            quote! {
                fn fixed_size() -> Option<usize>
                {
                    Some(#(<#types as reader_writer::Readable>::fixed_size().unwrap())+*)
                }
            }
        } else {
            let storage_idents = self.fields.iter()
                .filter(|field| field.has_storage())
                .map(|field| &field.ident);
            let tys = self.fields.iter().filter_map(|field| field.write_type());
            let exprs = self.fields.iter().filter_map(|field| field.write_expr());

            quote! {
                fn size(&self) -> usize
                {
                    let mut __sum__ = 0;
                    let #name { #(#storage_idents,)* } = self;

                    #(__sum__ += <#tys as reader_writer::Readable>::size(&(#exprs));)*
                    __sum__
                }
            }
        };

        let args_ty = &self.args_ty;
        let args_pat = &self.args_pat;

        let idents = self.fields.iter().map(|field| &field.ident);
        let tys = self.fields.iter().map(|field| &field.ty);
        let read_exprs = self.fields.iter().map(|field| field.read_expr(name));

        let storage_idents = self.fields.iter()
            .filter(|field| field.has_storage())
            .map(|field| &field.ident);

        // The vast majority of structs won't contain padding and we can't guarantee that
        // querying the reader's remain length won't reqiure a syscall/context switch (which
        // can't be optimized out). Thus, only create __start__len__ if we're sure it's needed.
        let offset_let = if self.fields.iter().any(|field| field.needs_offset()) {
            quote!(let __start_len__ = __reader__.len(); )
        } else {
            proc_macro2::TokenStream::new()
        };

        quote! {
            #[automatically_derived]
            impl #impl_gens reader_writer::Readable<#reader_lifetime> for #name #type_gens
                #where_clause
            {
                type Args = #args_ty;
                fn read_from(
                    __reader__: &mut reader_writer::Reader<#reader_lifetime>,
                    #args_pat: Self::Args
                ) -> Self
                {
                    #offset_let
                    #(let #idents: #tys = #read_exprs;)*
                    #name {
                        #(#storage_idents,)*
                    }
                }

                #size_fn
            }
        }
    }

    // fn readable_size_fn_tokens(&self) -> proc_macro2::TokenStream
    // {
    // }

    // fn readable_fixed_size_fn_tokens(&self) -> proc_macro2::TokenStream
    // {
    // }

    fn writable_impl_tokens(&self) -> proc_macro2::TokenStream
    {
        let name = &self.struct_.ident;
        let (impl_gens, type_gens, where_clause) = self.struct_.generics.split_for_impl();

        let exprs = self.fields.iter().filter_map(|field| field.write_expr());
        let idents = self.fields.iter()
            .filter(|field| field.has_storage())
            .map(|field| &field.ident);
        let tys = self.fields.iter().filter_map(|field| field.write_type());

        quote! {
            #[automatically_derived]
            impl #impl_gens reader_writer::Writable for #name #type_gens
                #where_clause
            {
                fn write_to<W: std::io::Write>(&self, __writer__: &mut W)
                    -> std::io::Result<u64>
                {
                    let mut __sum__ = 0;
                    let #name { #(#idents,)* } = self;
                    #(__sum__ += <#tys as reader_writer::Writable>::write_to(&(#exprs), __writer__)?;)*
                    Ok(__sum__)
                }
            }
        }

    }

    fn struct_and_impl_tokens(self, options: DeriveOptions) -> proc_macro2::TokenStream
    {

        let readable_tokens = if options.readable {
            self.readable_impl_tokens(options.fixed_size)
        } else {
            proc_macro2::TokenStream::new()
        };

        let writable_tokens = if options.writable {
            self.writable_impl_tokens()
        } else {
            proc_macro2::TokenStream::new()
        };

        let struct_tokens = self.struct_.into_token_stream();
        quote! {
            #struct_tokens
            #readable_tokens
            #writable_tokens
        }
    }
}


#[proc_macro_attribute]
pub fn auto_struct(attr: TokenStream, tokens: TokenStream) -> TokenStream {
    if attr.is_empty() {
        return Error::new(Span::call_site(), "auto_struct attribute must have arguments")
                .to_compile_error().into();
    }
    let options = parse_macro_input!(attr as DeriveOptions);
    let decl = parse_macro_input!(tokens as syn::ItemStruct);

    let as_struct = match AutoStructDecl::from_struct(decl) {
        Ok(s) => s,
        Err(e) => return e.to_compile_error().into(),
    };

    as_struct.struct_and_impl_tokens(options).into()
}

