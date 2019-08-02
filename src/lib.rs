//!
//! Async stream API experiment that may be introduced as a language feature in the future.
//!
//! This crate provides useful features for streams, using unstable `async_await` and `generators`.
//!
//! ### \#\[for_await\]
//!
//! Processes streams using a for loop.
//!
//! This is a reimplement of [futures-await]'s `#[async]` for loops for futures 0.3 and is an experimental implementation of [the idea listed as the next step of async/await](https://github.com/rust-lang/rfcs/blob/master/text/2394-async_await.md#for-await-and-processing-streams).
//!
//! ```rust
//! #![feature(async_await, stmt_expr_attributes, proc_macro_hygiene)]
//! use futures::stream::Stream;
//! use futures_async_stream::for_await;
//!
//! async fn collect(stream: impl Stream<Item = i32>) -> Vec<i32> {
//!     let mut vec = Vec::new();
//!     #[for_await]
//!     for value in stream {
//!         vec.push(value);
//!     }
//!     vec
//! }
//! ```
//!
//! `value` has the `Item` type of the stream passed in. Note that async for loops can only be used inside of `async` functions, closures, blocks, `#[async_stream]` functions and `async_stream_block!` macros.
//!
//! *If you want to write the equivalent code without this API, you can write this by combining `while let` loop, `.await`, `pin_mut` macro, and `StreamExt::next()` method:*
//!
//! ```rust
//! #![feature(async_await)]
//! use futures::{
//!     pin_mut,
//!     stream::{Stream, StreamExt},
//! };
//!
//! async fn collect(stream: impl Stream<Item = i32>) -> Vec<i32> {
//!     let mut vec = Vec::new();
//!     pin_mut!(stream);
//!     while let Some(value) = stream.next().await {
//!         vec.push(value);
//!     }
//!     vec
//! }
//! ```
//!
//! ### \#\[async_stream\]
//!
//! Creates streams via generators.
//!
//! This is a reimplement of [futures-await]'s `#[async_stream]` for futures 0.3 and is an experimental implementation of [the idea listed as the next step of async/await](https://github.com/rust-lang/rfcs/blob/master/text/2394-async_await.md#generators-and-streams).
//!
//! ```rust
//! #![feature(async_await, generators)]
//! use futures::stream::Stream;
//! use futures_async_stream::async_stream;
//!
//! // Returns a stream of i32
//! #[async_stream(item = i32)]
//! async fn foo(stream: impl Stream<Item = String>) {
//!     #[for_await]
//!     for x in stream {
//!         yield x.parse().unwrap();
//!     }
//! }
//! ```
//!
//! `#[async_stream]` must have an item type specified via `item = some::Path` and the values output from the stream must be yielded via the `yield` expression.
//!
//!
//! *If you want to write the equivalent code without this API, you can write this by manually implementing the combinator:*
//!
//! ```rust
//! #![feature(async_await)]
//! use futures::{
//!     stream::Stream,
//!     ready,
//!     task::{Context, Poll},
//! };
//! use pin_utils::unsafe_pinned;
//! use std::pin::Pin;
//!
//! fn foo<S>(stream: S) -> impl Stream<Item = i32>
//! where
//!     S: Stream<Item = String>,
//! {
//!     Foo { stream }
//! }
//!
//! struct Foo<S> {
//!     stream: S,
//! }
//!
//! impl<S> Foo<S> {
//!     unsafe_pinned!(stream: S);
//! }
//!
//! impl<S> Stream for Foo<S>
//! where
//!     S: Stream<Item = String>,
//! {
//!     type Item = i32;
//!
//!     fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
//!         if let Some(x) = ready!(self.stream().poll_next(cx)) {
//!             Poll::Ready(Some(x.parse().unwrap()))
//!         } else {
//!             Poll::Ready(None)
//!         }
//!     }
//! }
//! ```
//!
//! [futures-await]: https://github.com/alexcrichton/futures-await

#![doc(html_root_url = "https://docs.rs/futures-async-stream/0.1.0-alpha.1")]
#![doc(test(attr(deny(warnings), allow(dead_code, unused_assignments, unused_variables))))]
#![warn(rust_2018_idioms, unreachable_pub, single_use_lifetimes)]
#![warn(clippy::all, clippy::pedantic)]
#![feature(async_await, gen_future, generator_trait, generators)]

/// Processes streams using a for loop.
pub use futures_async_stream_macro::for_await;

/// Creates streams via generators.
pub use futures_async_stream_macro::async_stream;

/// Creates streams via generators.
pub use futures_async_stream_macro::async_stream_block;

#[doc(hidden)]
pub mod stream;

#[doc(hidden)]
pub mod core_reexport {
    #[doc(hidden)]
    pub use core::*;
}

#[doc(hidden)]
pub mod futures_reexport {
    #[doc(hidden)]
    pub use futures_core::*;
}
