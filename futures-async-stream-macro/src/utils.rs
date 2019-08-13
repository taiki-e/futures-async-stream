use std::mem;

use proc_macro2::{Span, TokenStream, TokenTree};
use quote::ToTokens;
use syn::{punctuated::Punctuated, token, Expr, ExprTuple};

pub(crate) fn first_last<T>(tokens: &T) -> (Span, Span)
where
    T: ToTokens,
{
    let mut spans = TokenStream::new();
    tokens.to_tokens(&mut spans);
    let good_tokens = spans.into_iter().collect::<Vec<_>>();
    let first_span = good_tokens.first().map_or_else(Span::call_site, TokenTree::span);
    let last_span = good_tokens.last().map_or_else(|| first_span, TokenTree::span);
    (first_span, last_span)
}

pub(crate) fn respan(input: TokenStream, (first_span, last_span): (Span, Span)) -> TokenStream {
    let mut new_tokens = input.into_iter().collect::<Vec<_>>();
    if let Some(token) = new_tokens.first_mut() {
        token.set_span(first_span);
    }
    for token in new_tokens.iter_mut().skip(1) {
        token.set_span(last_span);
    }
    new_tokens.into_iter().collect()
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

pub(super) fn replace_boxed_expr<F>(expr: &mut Option<Box<Expr>>, f: F)
where
    F: FnOnce(Expr) -> Expr,
{
    if expr.is_none() {
        expr.replace(Box::new(unit()));
    }

    if let Some(expr) = expr {
        replace_expr(&mut **expr, f);
    }
}

macro_rules! error {
    ($span:expr, $msg:expr) => {
        syn::Error::new_spanned($span, $msg)
    };
    ($span:expr, $($tt:tt)*) => {
        error!($span, format!($($tt)*))
    };
}
