extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{quote, quote_spanned, TokenStreamExt, ToTokens};
use syn::{
    braced, parenthesized, parse_macro_input, parse_quote, token, Error, Expr,
    Ident, LitByteStr, LitFloat, LitInt, Token, Result
};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

use std::collections::HashSet;

struct AsmBlock
{
    starting_address: Expr,
    _comma: Token![,],
    _brace: token::Brace,
    asm: Vec<AsmInstr>,
}

// XXX Why doesn't syn define this for us?
macro_rules! parse_quote_spanned {
    ($($tts:tt)*) => { syn::parse(quote_spanned! { $($tts)* }.into()).unwrap() };
}

impl Parse for AsmBlock
{
    fn parse(input: ParseStream) -> Result<Self>
    {
        let content;
        let block = AsmBlock {
            starting_address: input.parse()?,
            _comma: input.parse()?,
            _brace: braced!(content in input),
            asm: Punctuated::<AsmInstrVec, Token![;]>::parse_terminated(&content)?
                .into_iter()
                .flat_map(|aiv| aiv.0)
                .collect(),
        };

        // Detect duplicate labels
        let mut seen_labels = HashSet::new();
        for instr in &block.asm {
            for label in &instr.labels {
                if !seen_labels.insert(label.clone()) {
                    Err(Error::new(label.span(), "Duplicate label"))?
                }
            }
        }
        Ok(block)
    }
}

impl ToTokens for AsmBlock
{
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream)
    {
        let sa: &Ident = &parse_quote! { __sa__ };

        let labels_let_iter = self.asm.iter()
            .enumerate()
            .flat_map(|(i, instr)| {
                let i = (i * 4) as u32;
                instr.labels.iter().map(move |id| quote!(let #id = #sa + #i;))
            });
        let labels_name_iter = self.asm.iter()
            .flat_map(|instr| instr.labels.iter());
        let labels_field_iter = self.asm.iter()
            .flat_map(|instr| instr.labels.iter());

        let instrs_iter = self.asm.iter()
            .enumerate()
            .map(|(i, instr)| {
                let iter = instr.parts.iter().map(|(width, op)| {
                    match op {
                        AsmOp::Expr(e) => {
                            quote_spanned! {e.span()=>
                                ppcasm::AsmInstrPart::new(#width, #e)
                            }
                        },
                        AsmOp::BranchExpr(e) => {
                            let instr_offset = i as i64 * 4;
                            quote_spanned! {e.span()=>
                                ppcasm::AsmInstrPart::new(
                                    #width,
                                    ((#e) as i64 - #sa as i64 - (#instr_offset)) >> 2
                                )
                            }
                        },
                        AsmOp::AtBranchExpr(e) => {
                            quote_spanned! {e.span()=>
                                ppcasm::AsmInstrPart::new(#width, (#e) >> 2)
                            }
                        },
                    }
                });
                quote! { ppcasm::AsmInstrPart::assemble(&[#(#iter),*]) }
            });

        let sa_e = &self.starting_address;
        tokens.append_all(quote! {
            {
                let #sa: u32 = #sa_e;
                use ::ppcasm::generic_array::arr;
                #(#labels_let_iter)*
                struct Labels {
                    #(#labels_name_iter: u32, )*
                }
                let __labels__ = Labels {
                    #(#labels_field_iter,)*
                };
                ppcasm::AsmBlock::new(__sa__, arr![u32; #(#instrs_iter),* ], __labels__)
            }
        })
    }
}

#[derive(Clone)]
enum AsmOp
{
    Expr(Expr),
    BranchExpr(Expr),
    AtBranchExpr(Expr),
}

impl ToTokens for AsmOp
{
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream)
    {
        match self {
            AsmOp::Expr(e) => e.to_tokens(tokens),
            _ => unreachable!(),
        }
    }
}

struct AsmInstrVec(Vec<AsmInstr>);

impl From<Vec<AsmInstr>> for AsmInstrVec
{
    fn from(vec: Vec<AsmInstr>) -> Self
    {
        AsmInstrVec(vec)
    }
}

struct AsmInstr
{
    labels: Vec<Ident>,
    parts: Vec<(u8, AsmOp)>,
}

const GPR_NAMES: &[&str] = &[
    "r0", "r1", "r2", "r3", "r4", "r5", "r6", "r7", "r8", "r9",
    "r10", "r11", "r12", "r13", "r14", "r15", "r16", "r17", "r18", "r19",
    "r20", "r21", "r22", "r23", "r24", "r25", "r26", "r27", "r28", "r29",
    "r30", "r31",
];

const FPR_NAMES: &[&str] = &[
    "f0", "f1", "f2", "f3", "f4", "f5", "f6", "f7", "f8", "f9",
    "f10", "f11", "f12", "f13", "f14", "f15", "f16", "f17", "f18", "f19",
    "f20", "f21", "f22", "f23", "f24", "f25", "f26", "f27", "f28", "f29",
    "f30", "f31",
];

macro_rules! flag_ident {
    ($dotname:ident, $id:ident) => { $id };
    ($dotname:ident, .) => { $dotname };
}


macro_rules! parse_part {
    ($dotname:ident, (?$flag:tt)) => {
        {
            let ident = flag_ident!($dotname, $flag);
            (1, AsmOp::Expr(parse_quote! { #ident }))
        }
    };
    ($dotname:ident, ($width:expr ; $value:ident)) => {
        ($width, $value)
    };
    ($dotname:ident, ($width:expr ; {$($value:tt)+})) => {
        {
            ($width, AsmOp::Expr(parse_quote! { $($value)+ }))
        }
    };
    ($dotname:ident, ($width:expr ; $value:expr)) => {
        {
            let v = &$value;
            ($width, AsmOp::Expr(parse_quote! { #v }))

        }
    };
    ($dotname:ident, $id:ident) => { $id.clone() };
}

fn parse_immediate(input: ParseStream) -> Result<Expr>
{
    let expr: syn::Expr = if input.peek(token::Brace) {
        let content;
        let _ = braced!(content in input);
        content.parse()?
    } else {
        let minus = input.parse::<Token![-]>().ok();
        if let Ok(id) = input.parse::<Ident>() {
            parse_quote_spanned! {id.span()=> #minus #id }
        } else {
            let lit: LitInt = input.parse()?;
            let v = lit.base10_parse::<i64>()?;
            parse_quote_spanned! {lit.span()=> #minus #v }
        }
    };
    if let Ok(_) = input.parse::<Token![@]>() {
        let id: Ident = input.parse()?;
        if id == "h" {
            Ok(parse_quote_spanned! {expr.span()=> ppcasm::upper_bits(#expr) })
        } else if id == "l" {
            Ok(parse_quote_spanned! {expr.span()=> ppcasm::lower_bits(#expr) })
        } else {
            Err(Error::new(id.span(), "Expected either 'h' or 'l'"))
        }
    } else {
        Ok(expr)
    }
}

macro_rules! parse_operand {
    ($input:ident, (r:$i:ident:$d:ident)) => {
        let $d = AsmOp::Expr(parse_immediate($input)?);
        let content;
        let _: token::Paren = parenthesized!(content in $input);
        parse_operand!(content, (r:$i));
    };
    ($input:ident, (r:$i:ident)) => {
        let ident: Ident = $input.parse()?;
        let $i = if let Some(i) = GPR_NAMES.iter().position(|n| ident == n) {
            let i = i as i64;
            (5, AsmOp::Expr(parse_quote_spanned! {ident.span()=> #i }))
        } else {
            Err(Error::new(ident.span(), format!("Expected GP register name, got {}", ident)))?
        };
    };
    ($input:ident, (f:$i:ident)) => {
        let ident: Ident = $input.parse()?;
        let $i = if let Some(i) = FPR_NAMES.iter().position(|n| ident == n) {
            let i = i as i64;
            (5, AsmOp::Expr(parse_quote_spanned! {ident.span()=> #i }))
        } else {
            Err(Error::new(ident.span(), format!("Expected FP register name, got {}", ident)))?
        };
    };
    ($input:ident, (i:$i:ident)) => {
        let $i = AsmOp::Expr(parse_immediate($input)?);
    };
    ($input:ident, (l:$i:ident)) => {
        let ctor = if let Ok(_) = $input.parse::<Token![@]>() {
            AsmOp::AtBranchExpr
        } else {
            AsmOp::BranchExpr
        };
        let $i = ctor(parse_immediate($input)?);
    };
}

mod kw {
    syn::custom_keyword!(float);
    syn::custom_keyword!(long);
    syn::custom_keyword!(asciiz);
}

macro_rules! decl_instrs {
    ($( $nm:ident $([$flag:tt])* $(, $arg:tt)* => $($part:tt)|* ;)+) => {
        impl Parse for AsmInstrVec
        {
            fn parse(input: ParseStream) -> Result<Self>
            {
                let mut labels = vec![];
                // XXX Peeking for Idents is way too dumb :(
                while input.peek(|_| -> Ident { unreachable!() }) && input.peek2(Token![:]) {
                    labels.push(input.parse()?);
                    let _: Token![:] = input.parse()?;
                }

                if let Ok(_) = input.parse::<Token![.]>() {
                    let e = if let Ok(_) = input.parse::<kw::float>() {
                        if let Ok(lit) = input.parse::<LitFloat>() {
                            let f = lit.base10_parse::<f32>()?;
                            parse_quote_spanned! {lit.span()=>#f.to_bits() }
                        } else {
                            let expr = input.parse::<Expr>()?;
                            parse_quote_spanned! {expr.span()=>(#expr).to_bits() }
                        }
                    } else if let Ok(_) = input.parse::<kw::long>() {
                        input.parse()?

                    } else if let Ok(_) = input.parse::<kw::asciiz>() {
                        let lit = input.parse::<LitByteStr>()?;
                        let mut bytes = lit.value();
                        bytes.push(0);
                        while bytes.len() % 4 != 0 {
                            bytes.push(0);
                        }
                        let mut labels = Some(labels);
                        return Ok(bytes.chunks(4)
                                .map(|chunk| AsmInstr {
                                    labels: labels.take().unwrap_or(vec![]),
                                    parts: chunk.iter()
                                        .map(|b| *b)
                                        .map(|b| parse_quote_spanned! {lit.span()=> #b })
                                        .map(|e| (8, AsmOp::Expr(e)))
                                        .collect(),
                                })
                                .collect::<Vec<_>>()
                                .into()
                            );

                    } else {
                        Err(input.error("Unsupported directive"))?
                    };
                    return Ok(vec![AsmInstr {
                        labels,
                        parts: vec![(32, AsmOp::Expr(e))],
                    }].into());
                }

                let ident: Ident = input.parse()?;
                let maybe_dot = input.parse::<Token![.]>();
                let opname = ident.to_string() + maybe_dot.as_ref().map(|_| ".").unwrap_or("");

                $(
                if opname.starts_with(stringify!($nm)) {
                    let flags_str = &opname[stringify!($nm).len()..];
                    $(
                    let mut flags_str = flags_str;
                    let flag_ident!(__dot__, $flag) = if flags_str.starts_with(stringify!($flag)) {
                        flags_str = &{ flags_str }[stringify!($flag).len()..];
                        1i32
                    } else {
                        0i32
                    };
                    )*

                    if flags_str.len() == 0 {
                        let mut _first = true;
                        $(
                        if !_first {
                            let _comma: Token![,] = input.parse()?;
                        }
                        _first = false;
                        parse_operand!(input, $arg);
                        )*
                        let parts = vec![
                            $(parse_part!(__dot__, $part),)*
                        ];
                        return Ok(vec![AsmInstr {
                            labels,
                            parts
                        }].into());
                    }
                }
                )*
                Err(Error::new(ident.span(), format!("Invalid opcode {}", opname)))
            }
        }
    };
}

// const BO_FALSE: i32 = 4;
// const BO_TRUE: i32 = 12;

decl_instrs! {
    add[o][.],  (r:d), (r:a), (r:b)     => (6;31) | d | a | b | (?o) | (9;266) | (?.);
    addc[o][.], (r:d), (r:a), (r:b)     => (6;31) | d | a | b | (?o) | (9;10) | (?.);
    addi,       (r:d), (r:a), (i:imm)   => (6;14) | d | a | (16;imm);
    addic[.],   (r:d), (r:a), (i:imm)   => (5;6) | (?.) | d | a | (16;imm);
    b[l][a],    (l:li)                  => (6;18) | (24;li) | (?a) | (?l);
    blr                                 => (32;0x4e800020);
    blt[l][a],  (l:li)                  => (6;16) | (5;12) | (5;0) | (14;li) | (?a) | (?l);
    bge[l][a],  (l:li)                  => (6;16) | (5;4)  | (5;0) | (14;li) | (?a) | (?l);
    bgt[l][a],  (l:li)                  => (6;16) | (5;12) | (5;1) | (14;li) | (?a) | (?l);
    ble[l][a],  (l:li)                  => (6;16) | (5;4)  | (5;1) | (14;li) | (?a) | (?l);
    beq[l][a],  (l:li)                  => (6;16) | (5;12) | (5;2) | (14;li) | (?a) | (?l);
    bne[l][a],  (l:li)                  => (6;16) | (5;4)  | (5;2) | (14;li) | (?a) | (?l);
    bso[l][a],  (l:li)                  => (6;16) | (5;12) | (5;3) | (14;li) | (?a) | (?l);
    bns[l][a],  (l:li)                  => (6;16) | (5;4)  | (5;3) | (14;li) | (?a) | (?l);
    cmplwi,     (r:a), (i:imm)          => (6;10) | (3;0) | (1;0) | (1;0) | a | (16;imm);
    cntlzw[.],  (r:a), (r:s)            => (6;31) | s | a | (5;0) | (10;26) | (?.);
    lbz,        (r:d), (r:a:dis)        => (6;34) | d | a | (16;dis);
    lfs,        (f:d), (r:a:dis)        => (6;48) | d | a | (16;dis);
    lfsx,       (f:d), (r:a), (r:b)     => (6;31) | d | a | b | (10;535) | (1;0);
    li,         (r:d), (i:imm)          => (6;14) | d | (5;0) | (16;imm);
    lis,        (r:d), (i:imm)          => (6;15) | d | (5;0) | (16;imm);
    lha,        (r:d), (r:a:dis)        => (6;42) | d | a | (16;dis);
    lhz,        (r:d), (r:a:dis)        => (6;40) | d | a | (16;dis);
    lwz,        (r:d), (r:a:dis)        => (6;32) | d | a | (16;dis);
    lwzx,       (r:d), (r:a), (r:b)     => (6;31) | d | a | b | (10;23) | (1;0);
    mflr,       (r:d)                   => (6;31) | d | (10;0x100) | (10;339) | (1;0);
    mr,         (r:a), (r:s)            => (6;31) | s | a | s | (10;444) | (1;0);
    mtlr,       (r:d)                   => (6;31) | d | (10;0x100) | (10;467) | (1;0);
    mullw[o][.],(r:d), (r:a), (r:b)     => (6;31) | d | a | b | (?o) | (9;235) | (?.);
    nop                                 => (32;0x60000000);
    slwi,       (r:a), (r:s), (i:n)     => (6;21) | s | a | (5;{#n}) | (5;0) |(5;{31 - #n}) | (1;0);
    srwi,       (r:a), (r:s), (i:n)     => (6;21) | s | a | (5;{32 - #n}) | (5;n) |(5;31) | (1;0);
    rlwimi[.],  (r:a), (r:s), (i:sh), (i:mb), (i:me) =>
        (6;20) | s | a | (5;sh) | (5;mb) |(5;me) | (?.);
    rlwinm[.],  (r:a), (r:s), (i:sh), (i:mb), (i:me) =>
        (6;21) | s | a | (5;sh) | (5;mb) |(5;me) | (?.);
    stfs,       (f:d), (r:a:dis)        => (6;52) | d | a | (16;dis);
    stw,        (r:s), (r:a:dis)        => (6;36) | s | a | (16;dis);
    stwu,       (r:s), (r:a:dis)        => (6;37) | s | a | (16;dis);
    subf[o][.], (r:d), (r:a), (r:b)     => (6;31) | d | a | b | (?o) | (9;40) | (?.);
}

#[proc_macro]
pub fn ppcasm(tokens: TokenStream) -> TokenStream {
    let block = parse_macro_input!(tokens as AsmBlock);
    block.into_token_stream().into()
}
