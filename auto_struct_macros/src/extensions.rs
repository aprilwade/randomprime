
use syntex_syntax::abi::Abi;
use syntex_syntax::ast::*;
use syntex_syntax::codemap::{Span, Spanned};
use syntex_syntax::ext::base::ExtCtxt;
use syntex_syntax::ext::build::AstBuilder;
use syntex_syntax::ext::quote::rt::DUMMY_SP;
use syntex_syntax::parse::token::InternedString;
use syntex_syntax::ptr::P;
use syntex_syntax::tokenstream::TokenTree;

pub trait ExtCtxtExt
{
    fn impl_item_ty(&self, span: Span, vis: Visibility, ident: Ident, ty: P<Ty>) -> ImplItem;
    fn impl_item_method(&self, span: Span, vis: Visibility, ident: Ident,
                        sig: MethodSig, block: P<Block>) -> ImplItem;
    fn struct_field(&self, span: Span, ident: Ident, ty: P<Ty>) -> StructField;
    fn method_sig(&self, unsafety: Unsafety, decl: P<FnDecl>, generics: Generics) -> MethodSig;

    fn empty_generics(&self) -> Generics
    {
        Generics {
            lifetimes: vec![],
            ty_params: P::new(),
            where_clause: WhereClause {
                id: DUMMY_NODE_ID,
                predicates: vec![],
            },
            span: DUMMY_SP,
        }
    }

    fn arg_self(&self, kind: SelfKind) -> Arg;
    fn inline_attribute(&self) -> Attribute;
    fn expr_mac(&self, span: Span, path: Path, tts: Vec<TokenTree>) -> P<Expr>;

    fn expr_assign_op(&self, span: Span, binop: BinOpKind, l: P<Expr>, r: P<Expr>) -> P<Expr>;

    fn arg_pat(&self, pat: P<Pat>, ty: P<Ty>) -> Arg
    {
        Arg {
            ty: ty,
            pat: pat,
            id: DUMMY_NODE_ID,
        }
    }
}

impl<'a> ExtCtxtExt for ExtCtxt<'a>
{
    fn impl_item_ty(&self, span: Span, vis: Visibility, ident: Ident, ty: P<Ty>) -> ImplItem
    {
        ImplItem {
            id: DUMMY_NODE_ID,
            ident: ident,
            vis: vis,
            defaultness: Defaultness::Final,
            attrs: vec![],
            node: ImplItemKind::Type(ty),
            span: span,
        }
    }

    fn impl_item_method(&self, span: Span, vis: Visibility, ident: Ident,
                        sig: MethodSig, block: P<Block>)
        -> ImplItem
    {
        ImplItem {
            id: DUMMY_NODE_ID,
            ident: ident,
            vis: vis,
            defaultness: Defaultness::Final,
            attrs: vec![],
            node: ImplItemKind::Method(sig, block),
            span: span,
        }
    }

    fn struct_field(&self, span: Span, ident: Ident, ty: P<Ty>) -> StructField
    {
        StructField {
                span: span,
                ident: Some(ident),
                vis: Visibility::Public,
                id: DUMMY_NODE_ID,
                ty: ty,
                attrs: vec![],
        }
    }

    fn method_sig(&self, unsafety: Unsafety, decl: P<FnDecl>, generics: Generics) -> MethodSig
    {
        MethodSig {
            unsafety: unsafety,
            constness: Spanned { node: Constness::NotConst, span: DUMMY_SP },
            abi: Abi::Rust,
            decl: decl,
            generics: generics,
        }
    }

    fn arg_self(&self, kind: SelfKind) -> Arg
    {
        Arg::from_self(ExplicitSelf { node: kind, span: DUMMY_SP },
                       SpannedIdent { node: self.ident_of("self"), span: DUMMY_SP })
    }

    fn inline_attribute(&self) -> Attribute
    {
        self.attribute(DUMMY_SP, self.meta_word(DUMMY_SP, InternedString::new("inline")))
    }

    fn expr_mac(&self, span: Span, path: Path, tts: Vec<TokenTree>) -> P<Expr>
    {
        self.expr(span, ExprKind::Mac(Mac { span: span, node: Mac_ { path: path, tts: tts } }))
    }

    fn expr_assign_op(&self, span: Span, binop: BinOpKind, l: P<Expr>, r: P<Expr>) -> P<Expr>
    {
        self.expr(span, ExprKind::AssignOp(Spanned { node: binop, span: DUMMY_SP }, l, r))
    }
}
