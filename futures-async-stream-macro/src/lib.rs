//! The futures-async-stream procedural macro implementations - **do not use directly**

#![doc(html_root_url = "https://docs.rs/futures-async-stream-macro/0.2.1")]
#![doc(test(
    no_crate_inject,
    attr(deny(warnings, rust_2018_idioms, single_use_lifetimes), allow(dead_code))
))]
#![warn(unsafe_code)]
#![warn(rust_2018_idioms, single_use_lifetimes, unreachable_pub)]
#![warn(clippy::all, clippy::default_trait_access)]
// https://github.com/rust-lang/rust-clippy/issues/5704
#![allow(clippy::unnested_or_patterns)]
#![feature(proc_macro_def_site)]

#[macro_use]
mod utils;

mod elision;
mod parse;
mod stream;
mod visitor;

use proc_macro::{Delimiter, Group, TokenStream, TokenTree};
use quote::ToTokens;
use syn::{parse_quote, token, Expr, ExprAsync, ExprForLoop};

use crate::utils::parse_as_empty;

/// Processes streams using a for loop.
///
/// See [crate level documentation][for_await] for details.
///
/// [for_await]: crate#for_await
#[proc_macro_attribute]
pub fn for_await(args: TokenStream, input: TokenStream) -> TokenStream {
    if let Err(e) = parse_as_empty(&args.into()) {
        return e.to_compile_error().into();
    }

    let mut expr: ExprForLoop = syn::parse_macro_input!(input);
    expr.attrs.insert(0, parse_quote!(#[for_await]));

    let mut expr = Expr::ForLoop(expr);
    visitor::Visitor::default().visit_for_loop(&mut expr);

    expr.into_token_stream().into()
}

/// Creates streams via generators.
///
/// See [crate level documentation][stream] for details.
///
/// [stream]: crate#stream
#[proc_macro_attribute]
pub fn stream(args: TokenStream, input: TokenStream) -> TokenStream {
    stream::attribute(args.into(), input.into(), parse::Context::Stream)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Creates streams via generators. This is equivalent to [`#[stream]`][stream] on async blocks.
///
/// [stream]: crate#stream
#[proc_macro]
pub fn stream_block(input: TokenStream) -> TokenStream {
    let input = TokenStream::from(TokenTree::Group(Group::new(Delimiter::Brace, input)));
    let block = syn::parse_macro_input!(input);
    let mut expr = ExprAsync {
        attrs: Vec::new(),
        async_token: token::Async::default(),
        capture: Some(token::Move::default()),
        block,
    };

    stream::parse_async(&mut expr, parse::Context::Stream)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Creates streams via generators.
///
/// See [crate level documentation][try_stream] for details.
///
/// [try_stream]: crate#try_stream
#[proc_macro_attribute]
pub fn try_stream(args: TokenStream, input: TokenStream) -> TokenStream {
    stream::attribute(args.into(), input.into(), parse::Context::TryStream)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Creates streams via generators. This is equivalent to [`#[try_stream]`][try_stream] on async blocks.
///
/// [try_stream]: crate#try_stream
#[proc_macro]
pub fn try_stream_block(input: TokenStream) -> TokenStream {
    let input = TokenStream::from(TokenTree::Group(Group::new(Delimiter::Brace, input)));
    let block = syn::parse_macro_input!(input);
    let mut expr = ExprAsync {
        attrs: Vec::new(),
        async_token: token::Async::default(),
        capture: Some(token::Move::default()),
        block,
    };

    stream::parse_async(&mut expr, parse::Context::TryStream)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
