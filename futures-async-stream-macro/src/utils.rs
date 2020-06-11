use proc_macro2::TokenStream;
use std::mem;
use syn::{
    punctuated::Punctuated, token, Attribute, Block, Error, Expr, ExprAsync, ExprTuple, Result,
    Token,
};

macro_rules! error {
    ($span:expr, $msg:expr) => {
        syn::Error::new_spanned(&$span, $msg)
    };
    ($span:expr, $($tt:tt)*) => {
        error!($span, format!($($tt)*))
    };
}

macro_rules! parse_quote_spanned {
    ($span:expr => $($tt:tt)*) => {
        syn::parse2(quote::quote_spanned!($span => $($tt)*)).unwrap_or_else(|e| panic!("{}", e))
    };
}

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
        attrs: Vec::new(),
        async_token: <Token![async]>::default(),
        capture: Some(<Token![move]>::default()),
        block,
    }
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
/// This is almost equivalent to `syn::parse2::<Nothing>()`, but produces
/// a better error message and does not require ownership of `tokens`.
pub(crate) fn parse_as_empty(tokens: &TokenStream) -> Result<()> {
    if tokens.is_empty() { Ok(()) } else { Err(error!(tokens, "unexpected token: {}", tokens)) }
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
                if attr.path.is_ident(ident) {
                    if prev.replace(i).is_some() {
                        return Err(error!(attr, "duplicate #[{}] attribute", ident));
                    }
                    parse_as_empty(&attr.tokens)?;
                }
                Ok((i + 1, prev))
            })
            .map(|(_, pos)| pos)
    }

    fn find(&self, ident: &str) -> Option<&Attribute> {
        self.iter().position(|attr| attr.path.is_ident(ident)).and_then(|i| self.get(i))
    }
}
