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
#![feature(proc_macro_def_site)]

#[macro_use]
mod utils;

mod elision;
mod stream;
mod visitor;

use proc_macro::{Delimiter, Group, TokenStream, TokenTree};
use quote::ToTokens;
use syn::{token, Expr, ExprAsync};

use crate::utils::parse_as_empty;

/// Processes streams using a for loop.
#[proc_macro_attribute]
pub fn for_await(args: TokenStream, input: TokenStream) -> TokenStream {
    if let Err(e) = parse_as_empty(&args.into()) {
        return e.to_compile_error().into();
    }

    let mut expr = match syn::parse_macro_input!(input) {
        Expr::ForLoop(mut expr) => {
            // FIXME: once https://github.com/rust-lang/rust/issues/43081 fixed,
            //        use `.insert(0, ..) instead.
            expr.attrs.push(syn::parse_quote!(#[for_await]));
            Expr::ForLoop(expr)
        }
        expr => {
            return error!(expr, "#[for_await] attribute may only be used on for loops")
                .to_compile_error()
                .into();
        }
    };

    visitor::Visitor::default().visit_for_loop(&mut expr);
    expr.into_token_stream().into()
}

/// Creates streams via generators.
#[proc_macro_attribute]
pub fn stream(args: TokenStream, input: TokenStream) -> TokenStream {
    stream::attribute(args.into(), input.into(), stream::Context::Stream)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Creates streams via generators. This is equivalent to `#[stream]` on async blocks.
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

    stream::parse_async(&mut expr, stream::Context::Stream)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Creates streams via generators.
#[proc_macro_attribute]
pub fn try_stream(args: TokenStream, input: TokenStream) -> TokenStream {
    stream::attribute(args.into(), input.into(), stream::Context::TryStream)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Creates streams via generators. This is equivalent to `#[try_stream]` on async blocks.
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

    stream::parse_async(&mut expr, stream::Context::TryStream)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
