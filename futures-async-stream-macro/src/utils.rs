use proc_macro2::{Span, TokenStream, TokenTree};
use quote::ToTokens;

pub(crate) fn first_last<T: ToTokens>(tokens: &T) -> (Span, Span) {
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

macro_rules! error {
    ($span:expr, $msg:expr) => {
        syn::Error::new_spanned($span, $msg)
    };
    ($span:expr, $($tt:tt)*) => {
        error!($span, format!($($tt)*))
    };
}
