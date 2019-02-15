
pub extern crate syntex;
extern crate syntex_syntax;

use syntex::Registry;

use syntex_syntax::ast::*;
use syntex_syntax::ext::build::AstBuilder;
use syntex_syntax::codemap::{Span, Spanned};
use syntex_syntax::ext::base::{ExtCtxt, MacEager, MacResult};
use syntex_syntax::ext::quote::rt::{DUMMY_SP, ExtParseUtils, ToTokens};
use syntex_syntax::tokenstream::TokenTree;
use syntex_syntax::ptr::P;
use syntex_syntax::parse::{token, PResult};
use syntex_syntax::parse::common::SeqSep;
use syntex_syntax::parse::parser::Parser;
use syntex_syntax::parse::token::{keywords, Comma};
use syntex_syntax::util::small_vector::SmallVector;

mod extensions;
use extensions::*;


pub fn register(registry: &mut Registry)
{
    registry.add_macro("auto_struct", expand_auto_struct_macro);
}

#[derive(Debug)]
struct FieldData
{
    //name: Ident,
    name: P<Pat>,
    ty: P<Ty>,
    args: P<Expr>,
    kind: FieldDataKind,
    span: Span,
}

impl FieldData
{
    fn fixed_size_method_expr(&self, cx: &ExtCtxt) -> P<Expr>
    {
        match self.kind {
            FieldDataKind::Args => panic!("Should not be able to reach here..."),
            FieldDataKind::Literal => panic!("Should not be able to reach here..."),
            FieldDataKind::Offset => panic!("Should not be able to reach here..."),
            _ => (),
        };
        let trait_path = cx.path_global(DUMMY_SP, vec![cx.ident_of("reader_writer"),
                                                       cx.ident_of("Readable")]);
        let (qpath, path) = cx.qpath(self.write_or_size_ty().clone(), trait_path,
                                     cx.ident_of("fixed_size"));
        let expr = cx.expr_qpath(DUMMY_SP, qpath, path);
        let expr = cx.expr_call(DUMMY_SP, expr, vec![]);
        cx.expr_method_call(DUMMY_SP, expr, cx.ident_of("unwrap"), vec![])
    }
    fn size_method_expr(&self, cx: &ExtCtxt) -> P<Expr>
    {
        let field_expr = match self.kind {
            FieldDataKind::Simple => cx.expr_ident(DUMMY_SP, get_pat_ident(&self.name)),
            FieldDataKind::Derivable(ref expr, _) => cx.expr_addr_of(DUMMY_SP, expr.clone()),
            FieldDataKind::Expected(ref expr) => cx.expr_addr_of(DUMMY_SP, expr.clone()),
            FieldDataKind::Args => panic!("Should not be able to reach here..."),
            FieldDataKind::Literal => panic!("Should not be able to reach here..."),
            FieldDataKind::Offset => panic!("Should not be able to reach here..."),
        };
        let trait_path = cx.path_global(DUMMY_SP, vec![cx.ident_of("reader_writer"),
                                                       cx.ident_of("Readable")]);
        let (qpath, path) = cx.qpath(self.write_or_size_ty().clone(), trait_path,
                                     cx.ident_of("size"));
        let method_expr = cx.expr_qpath(DUMMY_SP, qpath, path);
        cx.expr_call(DUMMY_SP, method_expr, vec![field_expr])
    }

    fn write_method_expr(&self, cx: &ExtCtxt) -> P<Expr>
    {
        let field_expr = match self.kind {
            FieldDataKind::Simple => cx.expr_ident(DUMMY_SP, get_pat_ident(&self.name)),
            FieldDataKind::Derivable(ref expr, _) => cx.expr_addr_of(DUMMY_SP, expr.clone()),
            FieldDataKind::Expected(ref expr) => cx.expr_addr_of(DUMMY_SP, expr.clone()),
            FieldDataKind::Args => panic!("Should not be able to reach here..."),
            FieldDataKind::Literal => panic!("Should not be able to reach here..."),
            FieldDataKind::Offset => panic!("Should not be able to reach here..."),
        };
        let trait_path = cx.path_global(DUMMY_SP, vec![cx.ident_of("reader_writer"),
                                                       cx.ident_of("Writable")]);
        let (qpath, path) = cx.qpath(self.write_or_size_ty().clone(), trait_path,
                                     cx.ident_of("write"));
        let method_expr = cx.expr_qpath(DUMMY_SP, qpath, path);
        let writer_expr = cx.expr_ident(DUMMY_SP, cx.ident_of("writer"));
        cx.expr_try(DUMMY_SP,
                    cx.expr_call(DUMMY_SP, method_expr, vec![field_expr, writer_expr]))
    }

    fn has_storage(&self) -> bool
    {
        match self.kind {
            FieldDataKind::Simple | FieldDataKind::Literal => true,
            _ => false,
        }
    }

    fn write_or_size_ty(&self) -> &P<Ty>
    {
        match self.kind {
            FieldDataKind::Simple => &self.ty,
            FieldDataKind::Derivable(_, ref ty) => ty.as_ref().unwrap_or(&self.ty),
            FieldDataKind::Expected(_) => &self.ty,
            FieldDataKind::Args => panic!("Should not be able to reach here..."),
            FieldDataKind::Literal => panic!("Should not be able to reach here..."),
            FieldDataKind::Offset => &self.ty,
        }
    }
}


#[derive(Debug)]
enum FieldDataKind
{
    Derivable(P<Expr>, Option<P<Ty>>),
    Expected(P<Expr>),
    Offset,
    Literal,
    Simple,
    Args,
}

impl FieldDataKind
{
    fn is_args(&self) -> bool
    {
        match *self {
            FieldDataKind::Args => true,
            _ => false,
        }
    }

    fn is_simple(&self) -> bool
    {
        match *self {
            FieldDataKind::Simple => true,
            _ => false,
        }
    }
}

fn get_pat_ident(pat: &P<Pat>) -> Ident
{
    match pat.node {
        PatKind::Ident(_, ref ident, _) => ident.node,
        _ => unreachable!(),
    }
}

fn parse_field_kind<'a>(parser: &mut Parser<'a>)
    -> PResult<'a, FieldDataKind>
{
    if parser.token != token::Pound {
        return Ok(FieldDataKind::Simple);
    }

    parser.bump();
    try!(parser.expect(&token::OpenDelim(token::Bracket)));
    let ident = try!(parser.parse_ident());
    let kind = if ident.name.as_str() == "derivable" {
        let ty = if parser.eat(&token::Colon) {
            Some(try!(parser.parse_ty()))
        } else {
            None
        };
        try!(parser.expect(&token::Eq));
        let expr = try!(parser.parse_expr());
        FieldDataKind::Derivable(expr, ty)
    } else if ident.name.as_str() == "expect" {
        try!(parser.expect(&token::Eq));
        let expr = try!(parser.parse_expr());
        FieldDataKind::Expected(expr)
    } else if ident.name.as_str() == "args" {
        FieldDataKind::Args
    } else if ident.name.as_str() == "literal" {
        FieldDataKind::Literal
    } else if ident.name.as_str() == "offset" {
        FieldDataKind::Offset
    } else {
        return Err(parser.diagnostic().struct_span_err(parser.last_span,
                                                        "Unknown field attribute"));
    };
    try!(parser.expect(&token::CloseDelim(token::Bracket)));
    Ok(kind)
}

fn parse_field<'a>(cx: &ExtCtxt, parser: &mut Parser<'a>)
    -> PResult<'a, SmallVector<FieldData>>
{
    let kind = try!(parse_field_kind(parser));

    let span = parser.span;
    let name = if kind.is_args() {
        try!(parser.parse_pat())
    } else {
        cx.pat_ident(DUMMY_SP, try!(parser.parse_ident()))
    };
    if kind.is_simple() && parser.eat(&token::Not) {
        return parse_alignment_padding(cx, parser, span, name);
    }
    try!(parser.expect(&token::Colon));
    let ty = try!(parser.parse_ty());
    let args = if !kind.is_args() && parser.eat(&token::Eq) {
        try!(parser.parse_expr())
    } else {
        cx.parse_expr("()".to_string())
    };

    Ok(SmallVector::one(FieldData {
        name: name,
        ty: ty,
        args: args,
        kind: kind,
        span: span,
    }))
}

fn parse_alignment_padding<'a>(cx: &ExtCtxt, parser: &mut Parser<'a>, span: Span, name: P<Pat>)
    -> PResult<'a, SmallVector<FieldData>>
{
    let name = match name.node {
        PatKind::Ident(_, ref ident, _) => format!("{}", ident.node),
        _ => panic!(),
    };
    if name != "alignment_padding" {
        return Err(parser.diagnostic().struct_span_err(span,
                &format!("unknown macro field \"{}\"", name)));
    }
    let exprs = parser.parse_unspanned_seq(
        &token::OpenDelim(token::Paren),
        &token::CloseDelim(token::Paren),
        SeqSep::trailing_allowed(token::Comma),
        |parser| parser.parse_expr()
    )?;
    if exprs.len() < 1 || exprs.len() > 2 {
        return Err(parser.diagnostic().struct_span_err(span,
                "expected one or two arguments"));
    }
    let alignment_expr = exprs[0].clone();
    /* let byte_expr = exprs.get(1)
        .map(|i| i.clone())
        .unwrap_or_else(|| cx.expr_usize(DUMMY_SP, 0));*/
    return Ok(SmallVector::many(vec![
        FieldData {
            name: cx.pat_ident(DUMMY_SP, cx.ident_of("_padding_offset")),
            ty: cx.ty_ident(DUMMY_SP, cx.ident_of("usize")),
            args: cx.parse_expr("()".to_string()),
            kind: FieldDataKind::Offset,
            span: DUMMY_SP,
        },
        FieldData {
            name: cx.pat_ident(DUMMY_SP, cx.ident_of("_padding")),
            ty: cx.ty_path(cx.path_all(
                DUMMY_SP,
                true,
                vec![cx.ident_of("reader_writer"), cx.ident_of("RoArray")],
                vec![],
                vec![cx.ty_ident(DUMMY_SP, cx.ident_of("u8"))],
                vec![],
            )),
            args: cx.expr_tuple(DUMMY_SP, vec![
                cx.expr_call_global(
                    DUMMY_SP,
                    vec![
                        cx.ident_of("reader_writer"),
                        cx.ident_of("pad_bytes_count"),
                    ],
                    vec![
                        alignment_expr.clone(),
                        cx.expr_ident(DUMMY_SP, cx.ident_of("_padding_offset")),
                    ],
                ),
                cx.expr_tuple(DUMMY_SP, vec![]),
            ]),
            kind: FieldDataKind::Derivable(
                cx.expr_call_global(
                    DUMMY_SP,
                    vec![
                        cx.ident_of("reader_writer"),
                        cx.ident_of("pad_bytes"),
                    ],
                    vec![
                        alignment_expr,
                        cx.expr_ident(DUMMY_SP, cx.ident_of("_padding_offset")),
                    ],
                ),
                None,
            ),
            span: DUMMY_SP,
        },
    ]));
}

fn parse_attributes<'a>(span: Span, parser: &mut Parser<'a>)
    -> PResult<'a, (Vec<Attribute>, bool, bool, bool)>
{
    let attributes = try!(parser.parse_outer_attributes());
    let (auto_struct_attrs, other_attrs) : (Vec<_>, Vec<_>) = attributes.into_iter()
        .partition(|attr|
                match attr.node.value.node {
                    MetaItemKind::List(ref is, _) => is == "auto_struct",
                    _ => false,
                });

    if auto_struct_attrs.len() == 0 {
        return Err(parser.diagnostic().struct_span_err(span,
                "Couldn't find an `auto_struct` attribute."));
    }
    if auto_struct_attrs.len() > 1 {
        return Err(parser.diagnostic().struct_span_err(span,
                "Found multiple `auto_struct` attributes."));
    }
    let mut readable = false;
    let mut writable = false;
    let mut fixed_size = false;

    match auto_struct_attrs[0].node.value.node {
        MetaItemKind::List(_, ref nested) => {
            for n in nested.iter() {
                let inner_item = match n.node {
                    NestedMetaItemKind::MetaItem(ref item) => item,
                    _ => return Err(parser.diagnostic().struct_span_err( span,
                                    &format!("Unexpected `auto_struct` attribute item {:?}",
                                            n.node))),
                };
                match inner_item.node {
                    MetaItemKind::Word(ref is) if is == "Readable" => readable = true,
                    MetaItemKind::Word(ref is) if is == "Writable" => writable = true,
                    MetaItemKind::Word(ref is) if is == "FixedSize" => fixed_size = true,
                    _ => return Err(parser.diagnostic().struct_span_err( span,
                                    &format!("Unexpected `auto_struct` attribute item {:?}",
                                            inner_item.node))),
                }
            }
        },
        _ => unreachable!(),
    };
    Ok((other_attrs, readable, writable, fixed_size))
}

fn build_match(cx: &ExtCtxt, ident: Ident, fields: &[FieldData], expr: P<Expr>)
    -> P<Expr>
{
    let pats = fields.iter()
        .filter(|f| f.has_storage())
        .map(|f| FieldPat {
            ident: get_pat_ident(&f.name),
            pat: cx.pat_ident_binding_mode(DUMMY_SP, get_pat_ident(&f.name),
                                           BindingMode::ByRef(Mutability::Immutable)),
            is_shorthand: true
        })
        .map(|fp| Spanned { node: fp, span: DUMMY_SP, })
        .collect();
    let pat = cx.pat_struct(DUMMY_SP, cx.path_ident(DUMMY_SP, ident), pats);
    cx.expr_match(
        DUMMY_SP,
        cx.expr_deref(DUMMY_SP, cx.expr_self(DUMMY_SP)),
        vec![cx.arm(DUMMY_SP, vec![pat], expr)]
    )
}

fn build_read_method(cx: &ExtCtxt, struct_ident: Ident, fields: &[FieldData],
                     args_pat: P<Pat>, args_ty: P<Ty>, lifetime: Lifetime)
    -> ImplItem
{
    let reader_ty = cx.ty_path(cx.path_all(
        DUMMY_SP,
        true,
        vec![cx.ident_of("reader_writer"), cx.ident_of("Reader")],
        vec![lifetime],
        vec![],
        vec![],
    ));

    let reader_ident = cx.ident_of("__reader__");
    let sig_args = vec![
        cx.arg(DUMMY_SP, reader_ident, reader_ty.clone()),
        cx.arg_pat(args_pat, args_ty.clone()),
    ];
    let self_ty = cx.ty_ident(DUMMY_SP, cx.ident_of("Self"));

    let sig = cx.method_sig(
        Unsafety::Normal,
        cx.fn_decl(sig_args, cx.ty(DUMMY_SP, TyKind::Tup(vec![self_ty, reader_ty]))),
        cx.empty_generics()
    );

    let mut stmts = Vec::with_capacity(2 + fields.len());
    // Its much easier to insert `let mut __reader__ = __reader__;` shadowing the method's
    // parameter than to make the parameter mutable.
    stmts.push(cx.stmt_let(DUMMY_SP, true, reader_ident,
                           cx.expr_ident(DUMMY_SP, reader_ident)));

    let offset_start_ident = cx.ident_of("__start_len__");
    // If there are any #[offset] fields, we need to note the starting size of the
    // reader.
    if fields.iter().any(|f| match f.kind { FieldDataKind::Offset => true, _ => false }) {
        stmts.push(cx.stmt_let(DUMMY_SP, true, offset_start_ident,
                               cx.expr_method_call(DUMMY_SP, cx.expr_ident(DUMMY_SP, reader_ident),
                               cx.ident_of("len"), vec![])));
    };

    for f in fields {
        let expr = match f.kind {
            FieldDataKind::Literal => f.args.clone(),
            FieldDataKind::Offset => {
                // For an offset field, its value is the difference between the starting
                // length and the current length (ie the number of bytes read so far.)
                let start = cx.expr_ident(DUMMY_SP, offset_start_ident);
                let end = cx.expr_method_call(DUMMY_SP, cx.expr_ident(DUMMY_SP, reader_ident),
                                              cx.ident_of("len"), vec![]);
                cx.expr_binary(DUMMY_SP, BinOpKind::Sub, start, end)
            },
            _ => cx.expr_method_call(DUMMY_SP, cx.expr_ident(DUMMY_SP, reader_ident),
                                     cx.ident_of("read"), vec![f.args.clone()]),

        };
        stmts.push(cx.stmt_let_typed(
            DUMMY_SP,
            false,
            get_pat_ident(&f.name),
            f.ty.clone(),
            expr,
        ).unwrap());
        match f.kind {
            FieldDataKind::Expected(ref expr) => {
                let mut tokens = cx.expr_ident(DUMMY_SP, get_pat_ident(&f.name)).to_tokens(cx);
                tokens.push(TokenTree::Token(DUMMY_SP, Comma));
                tokens.append(&mut expr.to_tokens(cx));
                tokens.push(TokenTree::Token(DUMMY_SP, Comma));
                tokens.append(&mut cx.parse_tts(format!(
                    "\"\n(Deserializing field {}::{})\"",
                    struct_ident, get_pat_ident(&f.name)
                )));
                stmts.push(cx.stmt_semi(cx.expr_mac(
                    DUMMY_SP,
                    cx.path_ident(DUMMY_SP, cx.ident_of("assert_eq")),
                    tokens,
                )))
            },
            _ => (),
        }
    }

    let struct_field_inits = fields.iter()
        .filter(|f| f.has_storage())
        .map(|f| cx.field_imm(DUMMY_SP, get_pat_ident(&f.name),
                              cx.expr_ident(DUMMY_SP, get_pat_ident(&f.name))))
        .collect();
    let struct_expr = cx.expr_struct_ident(DUMMY_SP, struct_ident, struct_field_inits);
    stmts.push(cx.stmt_let(DUMMY_SP, false, cx.ident_of("res"), struct_expr));

    let res_expr = cx.expr_tuple(DUMMY_SP, vec![cx.expr_ident(DUMMY_SP, cx.ident_of("res")),
                                                cx.expr_ident(DUMMY_SP, reader_ident)]);
    stmts.push(cx.stmt_expr(res_expr));

    let impl_item = cx.impl_item_method(
        DUMMY_SP,
        Visibility::Inherited,
        cx.ident_of("read"),
        sig,
        cx.block(DUMMY_SP, stmts),
    );
    ImplItem { attrs: vec![cx.inline_attribute()], ..impl_item }
}

fn build_write_method(cx: &ExtCtxt, struct_ident: Ident, fields: &[FieldData])
    -> ImplItem
{
    let self_arg = cx.arg_self(SelfKind::Region(None, Mutability::Immutable));
    let generics = cx.new_parser_from_tts(&cx.parse_tts("<W: ::std::io::Write>".to_string()))
                        .parse_generics().unwrap();
    let writer_arg = cx.arg(
        DUMMY_SP,
        cx.ident_of("writer"),
        cx.ty_rptr(DUMMY_SP, cx.ty_ident(DUMMY_SP, cx.ident_of("W")), None, Mutability::Mutable)
    );
    let result_ty = cx.ty_path(cx.path_all(
        DUMMY_SP,
        true,
        ["std", "io", "Result"].iter().map(|s| cx.ident_of(s)).collect(),
        vec![],
        vec![cx.ty(DUMMY_SP, TyKind::Tup(vec![]))],
        vec![],
    ));
    let sig = cx.method_sig(
        Unsafety::Normal,
        cx.fn_decl(vec![self_arg, writer_arg], result_ty),
        generics
    );

    let stmts = fields.iter()
        .enumerate()
        .filter(|&(_, f)| match f.kind { FieldDataKind::Literal => false, _ => true })
        .map(|(i, f)| match f.kind {
            FieldDataKind::Offset => {
                // For an offset field, calculate the size of all of the preceding
                // fields.
                // TODO: If we keep track of the previous offset, we would only have to
                //       add the sizes of most recent fields (assuming there are multiple
                //       #[offset] fields.)
                // XXX There's probably a more efficient way to do this. For example,
                //     if write were fn write<W: Write + Seek>(...) then we could check
                //     the current file cursor position at the start and end.
                //     Alternatively, perhaps the API for Writer could be changed to
                //     include returning the number of bytes written
                let expr = fields[..i].iter()
                    .filter(|f| match f.kind {
                        FieldDataKind::Literal | FieldDataKind::Offset => false,
                        _ => true,
                    })
                    .map(|f| f.size_method_expr(cx))
                    .fold(cx.expr_usize(DUMMY_SP, 0),
                          |l, r| cx.expr_binary(DUMMY_SP, BinOpKind::Add, l, r));
                cx.stmt_let_typed(DUMMY_SP, false, get_pat_ident(&f.name), f.ty.clone(),
                                  expr).unwrap()
            },
            _ => cx.stmt_semi(f.write_method_expr(cx)),
        })
        .collect();

    let expr = build_match(cx, struct_ident, fields, cx.expr_block(cx.block(DUMMY_SP, stmts)));

    let impl_item = cx.impl_item_method(
        DUMMY_SP,
        Visibility::Inherited,
        cx.ident_of("write"),
        sig,
        cx.block(DUMMY_SP, vec![cx.stmt_semi(expr), cx.parse_stmt("Ok(())".to_string())]),
    );
    ImplItem { attrs: vec![cx.inline_attribute()], ..impl_item }
}

fn build_size_method(cx: &ExtCtxt, ident: Ident, fields: &[FieldData])
    -> ImplItem
{
    let self_arg = cx.arg_self(SelfKind::Region(None, Mutability::Immutable));
    let sig = cx.method_sig(
        Unsafety::Normal,
        cx.fn_decl(vec![self_arg], cx.ty_ident(DUMMY_SP, cx.ident_of("usize"))),
        cx.empty_generics()
    );

    let sum = cx.ident_of("__sum__");
    let mut stmts = vec![cx.stmt_let(DUMMY_SP, true, sum, cx.expr_usize(DUMMY_SP, 0))];
    stmts.extend(fields.iter()
        .filter(|f| match f.kind { FieldDataKind::Literal => false, _ => true })
        .filter_map(|f| match f.kind {
            // Literals don't cont
            FieldDataKind::Literal => None,
            // The current offset is simply __sum__
            FieldDataKind::Offset => Some(cx.stmt_let_typed(
                DUMMY_SP,
                false,
                get_pat_ident(&f.name),
                f.ty.clone(),
                cx.expr_ident(DUMMY_SP, sum)).unwrap()),
            _ => Some(cx.stmt_semi(cx.expr_assign_op(
                DUMMY_SP,
                BinOpKind::Add,
                cx.expr_ident(DUMMY_SP, sum),
                f.size_method_expr(cx)))),
        })
    );
    stmts.push(cx.stmt_expr(cx.expr_ident(DUMMY_SP, sum)));

    let expr = build_match(cx, ident, fields, cx.expr_block(cx.block(DUMMY_SP, stmts)));

    let impl_item = cx.impl_item_method(
        DUMMY_SP,
        Visibility::Inherited,
        cx.ident_of("size"),
        sig,
        cx.block(DUMMY_SP, vec![cx.stmt_expr(expr)]),
    );
    ImplItem { attrs: vec![cx.inline_attribute()], ..impl_item }
}

fn build_fixed_size_method(cx: &ExtCtxt, fields: &[FieldData])
    -> ImplItem
{
    let sig = cx.method_sig(
        Unsafety::Normal,
        cx.fn_decl(vec![], cx.ty_option(cx.ty_ident(DUMMY_SP, cx.ident_of("usize")))),
        cx.empty_generics()
    );

    let sum = cx.ident_of("__sum__");
    let mut stmts = vec![cx.stmt_let(DUMMY_SP, true, sum, cx.expr_usize(DUMMY_SP, 0))];
    stmts.extend(fields.iter()
        .filter(|f| match f.kind { FieldDataKind::Literal | FieldDataKind::Offset => false,
                                   _ => true })
        .map(|f| f.fixed_size_method_expr(cx))
        .map(|e| cx.expr_assign_op(DUMMY_SP, BinOpKind::Add, cx.expr_ident(DUMMY_SP, sum), e))
        .map(|e| cx.stmt_semi(e))
    );
    stmts.push(cx.stmt_expr(cx.expr_ident(DUMMY_SP, sum)));
    let expr = cx.expr_block(cx.block(DUMMY_SP, stmts));

    let impl_item = cx.impl_item_method(
        DUMMY_SP,
        Visibility::Inherited,
        cx.ident_of("fixed_size"),
        sig,
        cx.block(DUMMY_SP, vec![cx.stmt_expr(cx.expr_some(DUMMY_SP, expr))]),
    );
    ImplItem { attrs: vec![cx.inline_attribute()], ..impl_item }
}

fn build_readable_impl<'cx>(cx: &'cx ExtCtxt, fixed_size: bool, struct_ident: Ident,
                            fields: &[FieldData], args_pat: P<Pat>, args_ty: P<Ty>,
                            generics: &Generics)
    -> PResult<'cx, P<Item>>
{
    let mut impl_generics = generics.clone();
    let impl_lifetime = if generics.lifetimes.len() == 0 {
        impl_generics.lifetimes.push( cx.lifetime_def(DUMMY_SP, cx.name_of("'reader"), vec![]));
        cx.lifetime(DUMMY_SP, cx.name_of("'reader"))
    } else if generics.lifetimes.len() == 1 {
        generics.lifetimes[0].lifetime
    } else {
        return Err(cx.struct_span_err(generics.span,
                    "At most 1 lifetime parameter is supported at this time"))
    };

    let trait_ref = cx.trait_ref(cx.path_all(
        DUMMY_SP,
        true,
        vec![cx.ident_of("reader_writer::Readable")],
        vec![impl_lifetime],
        vec![], vec![]
    ));
    let struct_ty = cx.ty_path(cx.path_all(
        DUMMY_SP,
        false,
        vec![struct_ident],
        generics.lifetimes.iter().map(|ldef| ldef.lifetime).collect(),
        generics.ty_params.iter().map(|ty_param| cx.ty_ident(DUMMY_SP, ty_param.ident)).collect(),
        vec![],
    ));

    let size_method = if fixed_size {
        build_fixed_size_method(cx, &fields)
    } else {
        build_size_method(cx, struct_ident, &fields)
    };
    let impl_items = vec![
        cx.impl_item_ty(DUMMY_SP, Visibility::Inherited, cx.ident_of("Args"), args_ty.clone()),
        build_read_method(cx, struct_ident, &fields, args_pat, args_ty, impl_lifetime),
        size_method,
    ];
    let impl_def = ItemKind::Impl(Unsafety::Normal, ImplPolarity::Positive, impl_generics,
                                  Some(trait_ref), struct_ty, impl_items);
    Ok(cx.item(DUMMY_SP, cx.ident_of(""), vec![], impl_def))
}


fn build_writable_impl<'cx>(cx: &'cx ExtCtxt, struct_ident: Ident, fields: &[FieldData],
                            generics: &Generics)
    -> PResult<'cx, P<Item>>
{
    let trait_ref = cx.trait_ref(cx.path_all(
        DUMMY_SP,
        true,
        vec![cx.ident_of("reader_writer::Writable")],
        vec![], vec![], vec![]
    ));
    let struct_ty = cx.ty_path(cx.path_all(
        DUMMY_SP,
        false,
        vec![struct_ident],
        generics.lifetimes.iter().map(|ldef| ldef.lifetime).collect(),
        generics.ty_params.iter().map(|ty_param| cx.ty_ident(DUMMY_SP, ty_param.ident)).collect(),
        vec![],
    ));

    let impl_items = vec![
        build_write_method(cx, struct_ident, fields)
    ];
    let impl_def = ItemKind::Impl(Unsafety::Normal, ImplPolarity::Positive, generics.clone(),
                                  Some(trait_ref), struct_ty, impl_items);
    Ok(cx.item(DUMMY_SP, cx.ident_of(""), vec![], impl_def))
}

fn parse_auto_struct<'cx>(cx: &'cx mut ExtCtxt, span: Span, parser: &mut Parser<'cx>)
    -> PResult<'cx, SmallVector<P<Item>>>
{
    let (attributes, readable, writable, fixed_size) = try!(parse_attributes(span, parser));

    let vis = if parser.eat_keyword(keywords::Pub) {
        Visibility::Public
    } else {
        Visibility::Inherited
    };
    try!(parser.expect_keyword(keywords::Struct));
    let ident = try!(parser.parse_ident());
    let mut generics = try!(parser.parse_generics());
    generics.where_clause = try!(parser.parse_where_clause());

    let fields = try!(parser.parse_unspanned_seq(
        &token::OpenDelim(token::Brace),
        &token::CloseDelim(token::Brace),
        SeqSep::trailing_allowed(token::Comma),
        |parser| parse_field(cx, parser),
    ));
    let fields: Vec<_> = fields.into_iter().flat_map(|i| i).collect();

    let (args_pat, args_ty, fields) = match fields.first() {
        Some(ref field) if field.kind.is_args() => (field.name.clone(), field.ty.clone(),
                                                    &fields[1..]),
        _ => (cx.pat_tuple(DUMMY_SP, vec![]), cx.ty(DUMMY_SP, TyKind::Tup(vec![])), &fields[..]),
    };
    for f in fields {
        if f.kind.is_args() {
            return Err(cx.struct_span_err(f.span,
                        "The #[args] attribute may only be applied to the first field."));
        }
    }


    let struct_fields = fields.iter()
        .filter(|f| f.has_storage())
        .map(|f| cx.struct_field(f.span, get_pat_ident(&f.name), f.ty.clone()))
        .collect();
    let struct_def = VariantData::Struct(struct_fields, DUMMY_NODE_ID);
    let struct_ = cx.item(span, ident, attributes,
                          ItemKind::Struct(struct_def, generics.clone()));
    let struct_ = struct_.map(|struct_| Item { vis:vis, ..struct_ });


    let mut vec = vec![struct_];
    if readable {
        vec.push(try!(build_readable_impl(cx, fixed_size, ident, fields, args_pat,
                                          args_ty, &generics)));
    }
    if writable {
        vec.push(try!(build_writable_impl(cx, ident, fields, &generics)));
    }

    Ok(SmallVector::many(vec))
}

fn expand_auto_struct_macro<'cx>(cx: &'cx mut ExtCtxt, span: Span, tts: &[TokenTree])
    -> Box<MacResult + 'cx>
{
    let mut parser = cx.new_parser_from_tts(tts);

    let res = parse_auto_struct(cx, span, &mut parser);

    match res {
        Err(mut diagnostic_builder) => {
            diagnostic_builder.emit();
            MacEager::items(SmallVector::zero())
        },
        Ok(items) => MacEager::items(items),
    }

}

#[cfg(test)]
mod tests
{
    use ::syntex;

    // TODO: Nicer/better tests...
    #[test]
    fn test_basic()
    {

        let mut registry = syntex::Registry::new();
        ::register(&mut registry);
        let res = registry.expand_str("input", "output", "
        auto_struct! {
            #[derive(Clone)]
            #[auto_struct(Readable, Writable)]
            pub struct Testing<'a, T: Test>
            {
                #[args]
                args: (u32, f32),

                #[literal]
                first: u32 = args.0,

                alignment_padding!(64),

                #[expect = 0xFF]
                magic: u32,
                #[derivable = array.len()]
                test: u32,
                array: Array<'a, u32> = (test, ()),
                #[derivable: Other = 10]
                bah: Bah,
                #[offset]
                offset: usize,
            }
        }
        ").unwrap();
        println!("{}", res);
    }
}
