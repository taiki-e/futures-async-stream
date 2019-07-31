//! Procedural macro for the `#[async_stream]` attribute.

#![recursion_limit = "256"]
#![doc(html_root_url = "https://docs.rs/futures-async-stream-macro/0.1.0-alpha.1")]
#![doc(test(attr(deny(warnings), allow(dead_code, unused_assignments, unused_variables))))]
#![warn(rust_2018_idioms, unreachable_pub, single_use_lifetimes)]
#![warn(clippy::all, clippy::pedantic)]

extern crate proc_macro;

use proc_macro::{Delimiter, Group, TokenStream, TokenTree};
use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;
use syn::{fold::Fold, Expr, ExprForLoop};

#[macro_use]
mod utils;

mod elision;
mod stream;
mod visitor;

/// Processes streams using a for loop.
#[proc_macro_attribute]
pub fn for_await(args: TokenStream, input: TokenStream) -> TokenStream {
    // TODO: Concurrent versions are not supported yet.
    if !args.is_empty() {
        return error!(TokenStream2::from(args), "attribute must be of the form `#[for_await]`")
            .to_compile_error()
            .into();
    }

    let mut expr: ExprForLoop = syn::parse_macro_input!(input);
    expr.attrs.push(syn::parse_quote!(#[for_await]));
    visitor::Visitor::default().fold_expr(Expr::ForLoop(expr)).into_token_stream().into()
}

/// Creates streams via generators.
#[proc_macro_attribute]
pub fn async_stream(args: TokenStream, input: TokenStream) -> TokenStream {
    stream::async_stream(args.into(), input.into()).unwrap_or_else(|e| e.to_compile_error()).into()
}

/// Creates streams via generators.
#[proc_macro]
pub fn async_stream_block(input: TokenStream) -> TokenStream {
    let input = TokenStream::from(TokenTree::Group(Group::new(Delimiter::Brace, input)));
    stream::async_stream_block(input.into()).unwrap_or_else(|e| e.to_compile_error()).into()
}
