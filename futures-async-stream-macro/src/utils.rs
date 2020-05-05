use std::mem;

use proc_macro2::TokenStream;
use syn::{punctuated::Punctuated, token, Block, Expr, ExprTuple, Result, Stmt};

macro_rules! error {
    ($span:expr, $msg:expr) => {
        syn::Error::new_spanned($span, $msg)
    };
    ($span:expr, $($tt:tt)*) => {
        error!($span, format!($($tt)*))
    };
}

pub(crate) fn block(stmts: Vec<Stmt>) -> Block {
    Block { brace_token: token::Brace::default(), stmts }
}

pub(crate) fn expr_compile_error(e: &syn::Error) -> syn::Expr {
    syn::parse2(e.to_compile_error()).unwrap()
}

pub(crate) fn unit() -> Expr {
    Expr::Tuple(ExprTuple {
        attrs: Vec::new(),
        paren_token: token::Paren::default(),
        elems: Punctuated::new(),
    })
}

pub(crate) fn replace_expr<F>(this: &mut Expr, f: F)
where
    F: FnOnce(Expr) -> Expr,
{
    *this = f(mem::replace(this, Expr::Verbatim(TokenStream::new())));
}

/// Check if `tokens` is an empty `TokenStream`.
/// This is almost equivalent to `syn::parse2::<Nothing>()`,
/// but produces a better error message and does not require ownership of `tokens`.
pub(crate) fn parse_as_empty(tokens: &TokenStream) -> Result<()> {
    if tokens.is_empty() { Ok(()) } else { Err(error!(tokens, "unexpected token: {}", tokens)) }
}
