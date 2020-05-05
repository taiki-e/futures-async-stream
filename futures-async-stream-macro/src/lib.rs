//! The futures-async-stream procedural macro implementations - **do not use directly**

#![recursion_limit = "256"]
#![doc(html_root_url = "https://docs.rs/futures-async-stream-macro/0.1.5")]
#![doc(test(
    no_crate_inject,
    attr(deny(warnings, rust_2018_idioms, single_use_lifetimes), allow(dead_code))
))]
#![warn(unsafe_code)]
#![warn(rust_2018_idioms, single_use_lifetimes, unreachable_pub)]
#![warn(clippy::all, clippy::default_trait_access)]

use proc_macro::{Delimiter, Group, TokenStream, TokenTree};
use quote::ToTokens;
use syn::{Expr, ExprForLoop};

#[macro_use]
mod utils;

mod elision;
mod stream;
mod try_stream;
mod visitor;

/// Processes streams using a for loop.
#[proc_macro_attribute]
pub fn for_await(args: TokenStream, input: TokenStream) -> TokenStream {
    if let Err(e) = utils::parse_as_empty(&args.into()) {
        return e.to_compile_error().into();
    }
    let mut expr: ExprForLoop = syn::parse_macro_input!(input);

    expr.attrs.push(syn::parse_quote!(#[for_await]));
    let mut expr = Expr::ForLoop(expr);
    visitor::Visitor::default().visit_for_loop(&mut expr);
    expr.into_token_stream().into()
}

/// Creates streams via generators.
#[proc_macro_attribute]
pub fn stream(args: TokenStream, input: TokenStream) -> TokenStream {
    stream::attribute(args.into(), input.into()).unwrap_or_else(|e| e.to_compile_error()).into()
}

/// Creates streams via generators.
#[proc_macro]
pub fn stream_block(input: TokenStream) -> TokenStream {
    let input = TokenStream::from(TokenTree::Group(Group::new(Delimiter::Brace, input)));
    stream::block_macro(input.into()).unwrap_or_else(|e| e.to_compile_error()).into()
}

/// Creates streams via generators.
#[proc_macro_attribute]
pub fn try_stream(args: TokenStream, input: TokenStream) -> TokenStream {
    try_stream::attribute(args.into(), input.into()).unwrap_or_else(|e| e.to_compile_error()).into()
}

/// Creates streams via generators.
#[proc_macro]
pub fn try_stream_block(input: TokenStream) -> TokenStream {
    let input = TokenStream::from(TokenTree::Group(Group::new(Delimiter::Brace, input)));
    try_stream::block_macro(input.into()).unwrap_or_else(|e| e.to_compile_error()).into()
}
