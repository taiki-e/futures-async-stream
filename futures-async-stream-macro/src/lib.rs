//! Implementation detail of the `futures-async-stream` crate. - **do not use directly**

#![doc(test(
    no_crate_inject,
    attr(
        deny(warnings, rust_2018_idioms, single_use_lifetimes),
        allow(dead_code, unused_variables)
    )
))]
#![warn(unsafe_code)]
#![warn(rust_2018_idioms, single_use_lifetimes, unreachable_pub, unsafe_op_in_unsafe_fn)]
#![warn(clippy::pedantic, clippy::undocumented_unsafe_blocks)]
#![feature(proc_macro_def_site)]

#[macro_use]
mod utils;

mod elision;
mod parse;
mod stream;
mod visitor;

use proc_macro::{Delimiter, Group, TokenStream, TokenTree};
use quote::ToTokens;
use syn::{parse_quote, Error, Expr, ExprForLoop};

use crate::utils::{expr_async, parse_as_empty};

/// Processes streams using a for loop.
///
/// See crate level documentation for details.
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
/// See crate level documentation for details.
#[proc_macro_attribute]
pub fn stream(args: TokenStream, input: TokenStream) -> TokenStream {
    stream::attribute(args.into(), input.into(), parse::Context::Stream)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

/// Creates streams via generators. This is equivalent to `#[stream]` on async blocks.
#[proc_macro]
pub fn stream_block(input: TokenStream) -> TokenStream {
    let input = TokenStream::from(TokenTree::Group(Group::new(Delimiter::Brace, input)));
    let block = syn::parse_macro_input!(input);
    let mut expr = expr_async(block);

    stream::parse_async(&mut expr, parse::Context::Stream).into()
}

/// Creates streams via generators.
///
/// See crate level documentation for details.
#[proc_macro_attribute]
pub fn try_stream(args: TokenStream, input: TokenStream) -> TokenStream {
    stream::attribute(args.into(), input.into(), parse::Context::TryStream)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

/// Creates streams via generators. This is equivalent to `#[try_stream]` on async blocks.
#[proc_macro]
pub fn try_stream_block(input: TokenStream) -> TokenStream {
    let input = TokenStream::from(TokenTree::Group(Group::new(Delimiter::Brace, input)));
    let block = syn::parse_macro_input!(input);
    let mut expr = expr_async(block);

    stream::parse_async(&mut expr, parse::Context::TryStream).into()
}
