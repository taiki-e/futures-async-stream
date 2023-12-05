// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::mem;

use proc_macro2::TokenStream;
use syn::{
    punctuated::Punctuated, token, Attribute, Block, Error, Expr, ExprAsync, ExprTuple, Result,
    Token,
};

macro_rules! def_site_ident {
    ($s:expr) => {
        syn::Ident::new($s, proc_macro::Span::def_site().into())
    };
    ($($tt:tt)*) => {
        quote::format_ident!($($tt)*, span = proc_macro::Span::def_site().into())
    };
}

pub(crate) fn expr_compile_error(e: &Error) -> Expr {
    syn::parse2(e.to_compile_error()).unwrap()
}

pub(crate) fn expr_async(block: Block) -> ExprAsync {
    ExprAsync {
        attrs: vec![],
        async_token: <Token![async]>::default(),
        capture: Some(<Token![move]>::default()),
        block,
    }
}

pub(crate) fn unit() -> Expr {
    Expr::Tuple(ExprTuple {
        attrs: vec![],
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

/// Checks if `tokens` is an empty `TokenStream`.
///
/// This is almost equivalent to `syn::parse2::<Nothing>()`, but produces
/// a better error message and does not require ownership of `tokens`.
pub(crate) fn parse_as_empty(tokens: &TokenStream) -> Result<()> {
    if tokens.is_empty() {
        Ok(())
    } else {
        bail!(tokens, "unexpected token: `{}`", tokens)
    }
}

// =================================================================================================
// extension traits

pub(crate) trait SliceExt {
    fn position_exact(&self, ident: &str) -> Result<Option<usize>>;
    fn find(&self, ident: &str) -> Option<&Attribute>;
}

impl SliceExt for [Attribute] {
    fn position_exact(&self, ident: &str) -> Result<Option<usize>> {
        self.iter()
            .try_fold((0, None), |(i, mut prev), attr| {
                if attr.path().is_ident(ident) {
                    if prev.replace(i).is_some() {
                        bail!(attr, "duplicate #[{}] attribute", ident);
                    }
                    attr.meta.require_path_only()?;
                }
                Ok((i + 1, prev))
            })
            .map(|(_, pos)| pos)
    }

    fn find(&self, ident: &str) -> Option<&Attribute> {
        self.iter().position(|attr| attr.path().is_ident(ident)).map(|i| &self[i])
    }
}
