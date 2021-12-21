extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    parenthesized,
    parse_macro_input,
    parse::Parser,
    punctuated::Punctuated,
    spanned::Spanned,
    Token,
};

use std::fmt;


struct NameExprPair
{
    ident: syn::Ident,
    _eq_token: Token![=],
    tokens: proc_macro2::TokenStream,
}

impl syn::parse::Parse for NameExprPair
{
    fn parse(input: syn::parse::ParseStream) -> syn::parse::Result<Self>
    {
        Ok(NameExprPair {
            ident: input.parse()?,
            _eq_token: input.parse()?,
            // XXX This is slightly hacky, but I'm not sure how to deal with the commas otherwise.
            tokens: input.parse::<syn::Expr>()?.into_token_stream(),
        })
    }
}

enum PatchKind
{
    Call,
    Return
}


struct Flags
{
    target: syn::LitStr,
    offset: syn::LitInt,
    version: syn::Expr,
    kind: PatchKind,
}

impl syn::parse::Parse for Flags
{
    fn parse(input: syn::parse::ParseStream) -> syn::parse::Result<Self>
    {
        let forked = input.fork();
        let pairs = Punctuated::<NameExprPair, Token![,]>::parse_terminated(input)?;

        let mut target_and_offset = None;
        let mut kind = None;
        let mut version = None;
        for pair in pairs {
            if pair.ident == "target" {
                if target_and_offset.is_some() {
                    Err(syn::Error::new(pair.ident.span(), "Duplicate `target` flag"))?;
                }
                let parser = |input: syn::parse::ParseStream| {
                    let name = input.parse()?;
                    let offset = if input.peek(Token![+]) {
                        let _plus: Token![+] = input.parse()?;
                        Some(input.parse()?)
                    } else {
                        None
                    };
                    Ok((name, offset))
                };
                let (name, offset) = parser.parse2(pair.tokens)?;
                let offset = offset.unwrap_or(syn::parse_quote!(0));
                target_and_offset = Some((name, offset))
            } else if pair.ident == "kind" {
                if kind.is_some() {
                    Err(syn::Error::new(pair.ident.span(), "Duplicate `kind` flag"))?;
                }
                let ident = <syn::Ident as syn::parse::Parse>::parse.parse2(pair.tokens)?;

                kind = if ident == "call" {
                    Some(PatchKind::Call)
                } else if ident == "return" {
                    Some(PatchKind::Return)
                } else {
                    Err(syn::Error::new_spanned(ident, "Unknown value for `kind` flag"))?
                }
            } else if pair.ident == "version" {
                if version.is_some() {
                    Err(syn::Error::new(pair.ident.span(), "Duplicate `version` flag"))?;
                }
                version = Some(<syn::Expr as syn::parse::Parse>::parse.parse2(pair.tokens)?);
            } else {
                Err(syn::Error::new(pair.ident.span(), "Unknown flag"))?;
            }
        }

        let kind = kind
            .ok_or_else(|| forked.error("Missing flag `kind`"))?;
        let (target, offset) = target_and_offset
            .ok_or_else(|| forked.error("Missing flag `target`"))?;
        let version = version.unwrap_or(syn::parse_quote!(Any));
        Ok(Flags { kind, target, offset, version })
    }
}

#[proc_macro_attribute]
pub fn patch_fn(attr: TokenStream, item: TokenStream) -> TokenStream
{
    let func = parse_macro_input!(item as syn::ItemFn);

    let flags = parse_macro_input!(attr as Flags);

    let patch_func_name = match flags.kind {
        PatchKind::Call => quote!(call_patch),
        PatchKind::Return => quote!(return_patch),
    };

    let offset = flags.offset;
    let target_func_name = flags.target;
    let version = flags.version;

    let func_name = &func.sig.ident;

    let hash_input = format!(
        "{:?}::{:?}::{:?}",
        target_func_name.value(),
        offset.to_token_stream(),
        version.to_token_stream(),
    );
    let hash = md5::compute(hash_input.as_bytes());

    let static_name = format!("PATCHES_{:x}_{}", hash, offset.base10_digits());
    let static_name = syn::parse_str::<syn::Ident>(&static_name).unwrap();

    (quote! {
        #[distributed_slice(primeapi::PATCHES)]
        static #static_name: primeapi::Patch = {
            extern "C" {
                #[link_name = #target_func_name]
                fn target_func();
            }

            primeapi::Patch::#patch_func_name(
                target_func as *const u8,
                #offset,
                #func_name as *const u8,
                { use primeapi::GameVersion::*; #version },
            )
        };

        #func
    }).into()
}

#[proc_macro_attribute]
pub fn prolog_fn(_attr: TokenStream, item: TokenStream) -> TokenStream
{
    let func = parse_macro_input!(item as syn::ItemFn);

    let func_name = &func.sig.ident;

    let static_name = format!("PROLOG_FUNCS_{}", func_name);
    let static_name = syn::parse_str::<syn::Ident>(&static_name).unwrap();

    (quote! {
        #[distributed_slice(primeapi::PROLOG_FUNCS)]
        static #static_name: unsafe extern "C" fn()  = #func_name;
        #func
    }).into()
}

enum CppBaseType
{
    Named(CppPath),
    Builtin(Option<cpp_kws::unsigned>, CppBuiltinType),
    // TODO: Function(Box<CppDeclType>, Punctuated<CppDeclType, token![,]>),
}

mod cpp_kws
{
    syn::custom_keyword!(unsigned);
    syn::custom_keyword!(new);

    syn::custom_keyword!(void);
    syn::custom_keyword!(bool);
    syn::custom_keyword!(char);
    syn::custom_keyword!(wchar_t);
    syn::custom_keyword!(short);
    syn::custom_keyword!(int);
    syn::custom_keyword!(long);
    syn::custom_keyword!(float);
    syn::custom_keyword!(double);
}

impl syn::parse::Parse for CppBaseType
{
    fn parse(input: syn::parse::ParseStream) -> syn::parse::Result<Self>
    {
        macro_rules! check_lookahead {
            ($(($ctor:ident, $tk:ident),)+) => {
                let maybe_unsigned = input.parse()?;
                if false {
                    unreachable!()
                }
                $(
                    else if input.peek(cpp_kws::$tk) {
                        let _: cpp_kws::$tk = input.parse()?;
                        Ok(CppBaseType::Builtin(maybe_unsigned, CppBuiltinType::$ctor))
                    }
                )+
                else if input.peek(cpp_kws::long) {
                    let _: cpp_kws::long = input.parse()?;
                    if input.peek(cpp_kws::long) {
                        let _: cpp_kws::long = input.parse()?;
                        Ok(CppBaseType::Builtin(maybe_unsigned, CppBuiltinType::LongLong))
                    } else {
                        Ok(CppBaseType::Builtin(maybe_unsigned, CppBuiltinType::Long))
                    }
                } else {
                    if let Some(unsigned) = maybe_unsigned {
                        Err(syn::Error::new(unsigned.span(), "Only builtin types maybe unsigned"))?
                    }
                    Ok(CppBaseType::Named(input.parse()?))
                }

            };
        }
        check_lookahead! {
            (Void, void),
            (Bool, bool),
            (Char, char),
            (WChar, wchar_t),
            (Short, short),
            (Int, int),
            (Float, float),
            (Double, double),
        }
    }
}

// https://github.com/Pwootage/prime-practice-native/blob/master/PrimeAPI/script/Mangle.py
// TODO: Special case constructors
impl fmt::Display for CppBaseType
{
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error>
    {
        match self {
            CppBaseType::Builtin(maybe_unsigned, ty) => {
                if maybe_unsigned.is_some() {
                    write!(f, "U")?;
                }
                let c = match ty {
                    CppBuiltinType::Void => 'v',
                    CppBuiltinType::Bool => 'b',
                    CppBuiltinType::Char => 'c',
                    CppBuiltinType::WChar => 'w',
                    CppBuiltinType::Short => 's',
                    CppBuiltinType::Int => 'i',
                    CppBuiltinType::Long => 'l',
                    CppBuiltinType::LongLong => 'x',
                    CppBuiltinType::Float => 'f',
                    CppBuiltinType::Double => 'd',
                };
                write!(f, "{}", c)?;
            },
            CppBaseType::Named(path) => write!(f, "{}", path)?,
        }
        Ok(())
    }
}

#[derive(Debug)]
enum CppBuiltinType
{
    Void,
    Bool,
    Char, WChar,
    Short,
    Int,
    Long,
    LongLong,
    Float,
    Double,
}

struct CppPtrQualifier(Token![*], Option<Token![const]>);

struct CppDeclType
{
    const_qual: Option<Token![const]>,
    base_type: CppBaseType,
    ptr_quals: Vec<CppPtrQualifier>,
    ref_qual: Option<Token![&]>,
}

impl syn::parse::Parse for CppDeclType
{
    fn parse(input: syn::parse::ParseStream) -> syn::parse::Result<Self>
    {
        Ok(CppDeclType {
            const_qual: input.parse()?,
            base_type: input.parse()?,
            ptr_quals: {
                let mut ptr_quals = vec![];
                while input.peek(Token![*]) {
                    ptr_quals.push(CppPtrQualifier(input.parse()?, input.parse()?))
                }
                ptr_quals
            },
            ref_qual: input.parse()?,
        })
    }
}

impl fmt::Display for CppDeclType
{
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error>
    {
        if self.ref_qual.is_some() {
            write!(f, "R")?;
        }
        for ptr_qual in self.ptr_quals.iter() {
            if ptr_qual.1.is_some() {
                write!(f, "C")?;
            }
            write!(f, "P")?;
        }
        if self.const_qual.is_some() {
            write!(f, "C")?;
        }

        write!(f, "{}", self.base_type)?;

        Ok(())
    }
}

enum CppOperatorType
{
    New,
    Add,
}


impl fmt::Display for CppOperatorType
{
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error>
    {
        match self {
            CppOperatorType::New => write!(f, "__nw"),
            CppOperatorType::Add => write!(f, "__pl"),
        }
    }
}

impl syn::parse::Parse for CppOperatorType
{
    fn parse(input: syn::parse::ParseStream) -> syn::parse::Result<Self>
    {
        let forked = input.fork();
        if let Ok(_) = input.parse::<cpp_kws::new>() {
            Ok(CppOperatorType::New)
        } else if let Ok(_) = input.parse::<Token![+]>() {
            Ok(CppOperatorType::Add)
        } else {
            Err(forked.error("Invalid operator type"))
        }
    }
}

struct CppPath(Punctuated<CppPathSegment, Token![::]>);// , Option<CppOperatorType>);
impl syn::parse::Parse for CppPath
{
    fn parse(input: syn::parse::ParseStream) -> syn::parse::Result<Self>
    {
        Ok(CppPath(Punctuated::<CppPathSegment, _>::parse_separated_nonempty(input)?))
    }
}

impl fmt::Display for CppPath
{
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error>
    {
        if self.0.len() > 1 {
            write!(f, "Q{}", self.0.len())?;

        }
        for seg in self.0.iter() {
            write!(f, "{}", seg)?;
        }
        Ok(())
    }
}

struct CppPathSegment
{
    id: syn::Ident,
    template_args: Option<CppTemplateArguments>,
}

impl syn::parse::Parse for CppPathSegment
{
    fn parse(input: syn::parse::ParseStream) -> syn::parse::Result<Self>
    {
        Ok(CppPathSegment {
            id: input.parse()?,
            template_args: if input.peek(Token![<]) {
                Some(input.parse()?)
            } else {
                None
            },
        })
    }
}

impl fmt::Display for CppPathSegment
{
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error>
    {
        if let Some(template_args) = &self.template_args {
            if f.alternate() {
                write!(f, "{}{}", self.id, template_args)
            } else {
                let ta_s = template_args.to_string();
                let id_s = self.id.to_string();
                write!(f, "{}{}{}", id_s.len() + ta_s.len(), id_s, ta_s)
            }
        } else {
            if f.alternate() {
                write!(f, "{}", self.id)
            } else {
                let s = self.id.to_string();
                write!(f, "{}{}", s.len(), s)
            }
        }
    }
}

struct CppTemplateArguments
{
    _left_angle_bracket: Token![<],
    params: Punctuated<CppDeclType, Token![,]>,
    _right_angle_bracket: Token![>],
}

impl fmt::Display for CppTemplateArguments
{
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error>
    {
        write!(f, "<")?;
        write!(f, "{}", self.params.first().unwrap())?;
        for param in self.params.iter().skip(1) {
            write!(f, ",{}", param)?;
        }
        write!(f, ">")?;
        Ok(())
    }
}

impl syn::parse::Parse for CppTemplateArguments
{
    fn parse(input: syn::parse::ParseStream) -> syn::parse::Result<Self>
    {
        Ok(CppTemplateArguments {
            _left_angle_bracket: input.parse()?,
            params: Punctuated::parse_separated_nonempty(input)?,
            _right_angle_bracket: input.parse()?,
        })
    }
}

struct CppFuncName
{
    path: CppPath,
    last_seg: CppPathSegment,
    operator_type: Option<CppOperatorType>,
}

impl syn::parse::Parse for CppFuncName
{
    fn parse(input: syn::parse::ParseStream) -> syn::parse::Result<Self>
    {
        let mut path: CppPath = input.parse()?;
        let last_seg = path.0.pop().unwrap().into_value();
        let operator_type = if last_seg.id == "operator" {
            Some(input.parse()?)
        } else {
            None
        };
        Ok(CppFuncName {
            path, last_seg, operator_type,
        })
    }
}

impl fmt::Display for CppFuncName
{
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error>
    {
        if self.last_seg.id == "operator" {
            write!(f, "{}__{}", self.operator_type.as_ref().unwrap(), self.path)?;
        } else {
            assert!(self.operator_type.is_none());
            write!(f, "{:#}__{}", self.last_seg, self.path)?;
        }
        Ok(())
    }
}

struct CppFuncDecl
{
    func_name: CppFuncName,
    _paren_token: syn::token::Paren,
    arguments: Punctuated<CppDeclType, Token![,]>,
    maybe_const: Option<Token![const]>,
}

impl syn::parse::Parse for CppFuncDecl
{
    fn parse(input: syn::parse::ParseStream) -> syn::parse::Result<Self>
    {
        let content;
        Ok(CppFuncDecl {
            func_name: input.parse()?,
            _paren_token: parenthesized!(content in input),
            arguments: Punctuated::parse_terminated(&content)?,
            maybe_const: input.parse()?,
        })
    }
}


impl fmt::Display for CppFuncDecl
{
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error>
    {
        write!(f, "{}", self.func_name)?;

        if self.maybe_const.is_some() {
            write!(f, "C")?;
        }
        write!(f, "F")?;

        for arg in self.arguments.iter() {
            write!(f, "{}", arg)?;
        }

        Ok(())
    }
}

#[proc_macro_attribute]
pub fn cw_link_name(attr: TokenStream, item: TokenStream) -> TokenStream
{
    let func = parse_macro_input!(item as syn::ForeignItemFn);

    let cpp_decl = parse_macro_input!(attr as CppFuncDecl);
    let mangled_name = cpp_decl.to_string();

    (quote! {
        #[link_name = #mangled_name]
        #func
    }).into()
}

#[proc_macro_attribute]
pub fn cpp_method(attr: TokenStream, fn_decl: TokenStream) -> TokenStream
{
    let func = parse_macro_input!(fn_decl as syn::ItemFn);

    let cpp_decl = parse_macro_input!(attr as CppFuncDecl);
    let mangled_name = cpp_decl.to_string();

    // let attrs = &func.attrs;
    let vis = &func.vis;
    let sig = &func.sig;
    if func.sig.unsafety.is_none() {
        // TODO: Error
    }

    let extern_ident = syn::Ident::new(&format!("{}_extern", sig.ident), sig.ident.span());
    let extern_sig = syn::Signature {
        ident: extern_ident.clone(),
        unsafety: None,
        ..(sig.clone())
    };

    let param_names = sig.inputs.iter()
        .map(|param| match param {
            syn::FnArg::Receiver(_) => syn::parse_quote!(self),
            syn::FnArg::Typed(pattype) => pattype.pat.clone(),
        });

    (quote! {
        #[inline(always)]
        #vis #sig
        {
            extern "C" {
                #[link_name = #mangled_name]
                #extern_sig;
            }
            #extern_ident(#(#param_names,)*)
        }
    }).into()
}
