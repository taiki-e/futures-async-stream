//! Async stream API experiment that may be introduced as a language feature in the future.
//!
//! This crate provides useful features for streams, using `async_await` and
//! unstable [`generators`](https://github.com/rust-lang/rust/issues/43122).
//!
//! # `#[for_await]`
//!
//! Processes streams using a for loop.
//!
//! This is a reimplement of [futures-await]'s `#[async]` for loops for
//! futures 0.3 and is an experimental implementation of [the idea listed as the
//! next step of async/await](https://github.com/rust-lang/rfcs/blob/HEAD/text/2394-async_await.md#for-await-and-processing-streams).
//!
//! ```rust
//! #![feature(proc_macro_hygiene, stmt_expr_attributes)]
//!
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
//! `value` has the `Item` type of the stream passed in. Note that async for
//! loops can only be used inside of `async` functions, closures, blocks,
//! `#[stream]` functions and `stream_block!` macros.
//!
//! # `#[stream]`
//!
//! Creates streams via generators.
//!
//! This is a reimplement of [futures-await]'s `#[stream]` for futures 0.3 and
//! is an experimental implementation of [the idea listed as the next step of
//! async/await](https://github.com/rust-lang/rfcs/blob/HEAD/text/2394-async_await.md#generators-and-streams).
//!
//! ```rust
//! #![feature(generators)]
//!
//! use futures::stream::Stream;
//! use futures_async_stream::stream;
//!
//! // Returns a stream of i32
//! #[stream(item = i32)]
//! async fn foo(stream: impl Stream<Item = String>) {
//!     // `for_await` is built into `stream`. If you use `for_await` only in `stream`, there is no need to import `for_await`.
//!     #[for_await]
//!     for x in stream {
//!         yield x.parse().unwrap();
//!     }
//! }
//! ```
//!
//! To early exit from a `#[stream]` function or block, use `return`.
//!
//! `#[stream]` on async fn must have an item type specified via
//! `item = some::Path` and the values output from the stream must be yielded
//! via the `yield` expression.
//!
//! `#[stream]` can also be used on async blocks:
//!
//! ```rust
//! #![feature(generators, proc_macro_hygiene, stmt_expr_attributes)]
//!
//! use futures::stream::Stream;
//! use futures_async_stream::stream;
//!
//! fn foo() -> impl Stream<Item = i32> {
//!     #[stream]
//!     async move {
//!         for i in 0..10 {
//!             yield i;
//!         }
//!     }
//! }
//! ```
//!
//! Note that `#[stream]` on async block does not require the `item` argument,
//! but it may require additional type annotations.
//!
//! # Using async stream functions in traits
//!
//! You can use async stream functions in traits by passing `boxed` or
//! `boxed_local` as an argument.
//!
//! ```rust
//! #![feature(generators)]
//!
//! use futures_async_stream::stream;
//!
//! trait Foo {
//!     #[stream(boxed, item = u32)]
//!     async fn method(&mut self);
//! }
//!
//! struct Bar(u32);
//!
//! impl Foo for Bar {
//!     #[stream(boxed, item = u32)]
//!     async fn method(&mut self) {
//!         while self.0 < u32::MAX {
//!             self.0 += 1;
//!             yield self.0;
//!         }
//!     }
//! }
//! ```
//!
//! A async stream function that received a `boxed` argument is converted to a
//! function that returns `Pin<Box<dyn Stream<Item = item> + Send + 'lifetime>>`.
//! If you passed `boxed_local` instead of `boxed`, async stream function
//! returns a non-threadsafe stream (`Pin<Box<dyn Stream<Item = item> + 'lifetime>>`).
//!
//! ```rust
//! #![feature(generators)]
//!
//! use std::pin::Pin;
//!
//! use futures::stream::Stream;
//! use futures_async_stream::stream;
//!
//! // The trait itself can be defined without unstable features.
//! trait Foo {
//!     fn method(&mut self) -> Pin<Box<dyn Stream<Item = u32> + Send + '_>>;
//! }
//!
//! struct Bar(u32);
//!
//! impl Foo for Bar {
//!     #[stream(boxed, item = u32)]
//!     async fn method(&mut self) {
//!         while self.0 < u32::MAX {
//!             self.0 += 1;
//!             yield self.0;
//!         }
//!     }
//! }
//! ```
//!
//! # `#[try_stream]`
//!
//! `?` operator can be used with the `#[try_stream]`. The `Item` of the
//! returned stream is `Result` with `Ok` being the value yielded and `Err` the
//! error type returned by `?` operator or `return Err(...)`.
//!
//! ```rust
//! #![feature(generators)]
//!
//! use futures::stream::Stream;
//! use futures_async_stream::try_stream;
//!
//! #[try_stream(ok = i32, error = Box<dyn std::error::Error>)]
//! async fn foo(stream: impl Stream<Item = String>) {
//!     #[for_await]
//!     for x in stream {
//!         yield x.parse()?;
//!     }
//! }
//! ```
//!
//! To early exit from a `#[try_stream]` function or block, use `return Ok(())`.
//!
//! # How to write the equivalent code without this API?
//!
//! ## `#[for_await]`
//!
//! You can write this by combining `while let` loop, `.await`, `pin_mut` macro,
//! and `StreamExt::next()` method:
//!
//! ```rust
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
//! ## `#[stream]`
//!
//! You can write this by manually implementing the combinator:
//!
//! ```rust
//! use std::pin::Pin;
//!
//! use futures::{
//!     ready,
//!     stream::Stream,
//!     task::{Context, Poll},
//! };
//! use pin_project::pin_project;
//!
//! fn foo<S>(stream: S) -> impl Stream<Item = i32>
//! where
//!     S: Stream<Item = String>,
//! {
//!     Foo { stream }
//! }
//!
//! #[pin_project]
//! struct Foo<S> {
//!     #[pin]
//!     stream: S,
//! }
//!
//! impl<S> Stream for Foo<S>
//! where
//!     S: Stream<Item = String>,
//! {
//!     type Item = i32;
//!
//!     fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
//!         if let Some(x) = ready!(self.project().stream.poll_next(cx)) {
//!             Poll::Ready(Some(x.parse().unwrap()))
//!         } else {
//!             Poll::Ready(None)
//!         }
//!     }
//! }
//! ```
//!
//! [futures-await]: https://github.com/alexcrichton/futures-await

#![no_std]
#![doc(test(
    no_crate_inject,
    attr(
        deny(warnings, rust_2018_idioms, single_use_lifetimes),
        allow(dead_code, unused_variables)
    )
))]
#![warn(
    missing_docs,
    rust_2018_idioms,
    single_use_lifetimes,
    unreachable_pub,
    unsafe_op_in_unsafe_fn
)]
#![warn(
    clippy::pedantic,
    // lints for public library
    clippy::alloc_instead_of_core,
    clippy::exhaustive_enums,
    clippy::exhaustive_structs,
    clippy::std_instead_of_alloc,
    clippy::std_instead_of_core,
    // lints that help writing unsafe code
    clippy::default_union_representation,
    clippy::trailing_empty_array,
    clippy::transmute_undefined_repr,
    clippy::undocumented_unsafe_blocks,
)]
#![allow(clippy::must_use_candidate)]
#![feature(generator_trait, gen_future)]

#[cfg(doctest)]
#[doc = include_str!("../README.md")]
const _README: () = ();

#[doc(inline)]
pub use futures_async_stream_macro::for_await;
#[doc(inline)]
pub use futures_async_stream_macro::stream;
#[doc(inline)]
pub use futures_async_stream_macro::stream_block;
#[doc(inline)]
pub use futures_async_stream_macro::try_stream;
#[doc(inline)]
pub use futures_async_stream_macro::try_stream_block;

mod future {
    use core::{
        future::Future,
        ops::{Generator, GeneratorState},
        pin::Pin,
        ptr::NonNull,
        task::{Context, Poll},
    };

    use pin_project::pin_project;

    // Based on https://github.com/rust-lang/rust/blob/1.63.0/library/core/src/future/mod.rs.

    /// This type is needed because:
    ///
    /// a) Generators cannot implement `for<'a, 'b> Generator<&'a mut Context<'b>>`, so we need to pass
    ///    a raw pointer (see <https://github.com/rust-lang/rust/issues/68923>).
    /// b) Raw pointers and `NonNull` aren't `Send` or `Sync`, so that would make every single future
    ///    non-Send/Sync as well, and we don't want that.
    ///
    /// It also simplifies the lowering of `.await`.
    #[doc(hidden)]
    #[derive(Debug, Copy, Clone)]
    pub struct ResumeTy(pub(crate) NonNull<Context<'static>>);

    // HACK: this is a hack to avoid trait bound error.
    // https://github.com/taiki-e/pin-project/issues/102#issuecomment-540472282
    #[doc(hidden)]
    pub struct Wrapper<'a, T>(core::marker::PhantomData<&'a ()>, T);

    // SAFETY: the caller of the `get_context` function that dereferences a
    // pointer must guarantee that no data races will occur.
    // Note: Since https://github.com/rust-lang/rust/pull/95985, `Context` is
    // `!Send` and `!Sync`, so implementing `Send` and `Sync` unconditionally here
    // is a bit subtle.
    unsafe impl<'a> Send for ResumeTy where Wrapper<'a, core::future::ResumeTy>: Send {}
    // SAFETY: see `Send` impl
    unsafe impl<'a> Sync for ResumeTy where Wrapper<'a, core::future::ResumeTy>: Sync {}

    /// Wrap a generator in a future.
    ///
    /// This function returns a `GenFuture` underneath, but hides it in `impl Trait` to give
    /// better error messages (`impl Future` rather than `GenFuture<[closure.....]>`).
    #[doc(hidden)]
    pub fn from_generator<G>(gen: G) -> impl Future<Output = G::Return>
    where
        G: Generator<ResumeTy, Yield = ()>,
    {
        GenFuture(gen)
    }

    #[pin_project]
    pub(crate) struct GenFuture<G>(#[pin] G);

    impl<G> Future for GenFuture<G>
    where
        G: Generator<ResumeTy, Yield = ()>,
    {
        type Output = G::Return;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let this = self.project();
            // Resume the generator, turning the `&mut Context` into a `NonNull` raw pointer. The
            // `.await` lowering will safely cast that back to a `&mut Context`.
            match this.0.resume(ResumeTy(NonNull::from(cx).cast::<Context<'static>>())) {
                GeneratorState::Yielded(()) => Poll::Pending,
                GeneratorState::Complete(x) => Poll::Ready(x),
            }
        }
    }

    #[doc(hidden)]
    pub unsafe fn get_context<'a, 'b>(cx: ResumeTy) -> &'a mut Context<'b> {
        // SAFETY: the caller must guarantee that `cx.0` is a valid pointer
        // that fulfills all the requirements for a mutable reference.
        unsafe { &mut *cx.0.as_ptr().cast() }
    }
}

mod stream {
    use core::{
        future::Future,
        ops::{Generator, GeneratorState},
        pin::Pin,
        ptr::NonNull,
        task::{Context, Poll},
    };

    use futures_core::stream::Stream;
    use pin_project::pin_project;

    use crate::future::ResumeTy;

    /// Wrap a generator in a stream.
    ///
    /// This function returns a `GenStream` underneath, but hides it in `impl Trait` to give
    /// better error messages (`impl Stream` rather than `GenStream<[closure.....]>`).
    #[doc(hidden)]
    pub fn from_generator<G, T>(gen: G) -> impl Stream<Item = T>
    where
        G: Generator<ResumeTy, Yield = Poll<T>, Return = ()>,
    {
        GenStream(gen)
    }

    #[pin_project]
    pub(crate) struct GenStream<G>(#[pin] G);

    impl<G, T> Stream for GenStream<G>
    where
        G: Generator<ResumeTy, Yield = Poll<T>, Return = ()>,
    {
        type Item = T;

        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            let this = self.project();
            match this.0.resume(ResumeTy(NonNull::from(cx).cast::<Context<'static>>())) {
                GeneratorState::Yielded(x) => x.map(Some),
                GeneratorState::Complete(()) => Poll::Ready(None),
            }
        }
    }

    // This is equivalent to the `futures::stream::StreamExt::next` method.
    // But we want to make this crate dependency as small as possible, so we define our `next` function.
    #[doc(hidden)]
    pub fn next<S>(stream: &mut S) -> impl Future<Output = Option<S::Item>> + '_
    where
        S: Stream + Unpin,
    {
        Next(stream)
    }

    pub(crate) struct Next<'a, S>(&'a mut S);

    impl<S> Future for Next<'_, S>
    where
        S: Stream + Unpin,
    {
        type Output = Option<S::Item>;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            Pin::new(&mut self.0).poll_next(cx)
        }
    }
}

mod try_stream {
    use core::{
        ops::{Generator, GeneratorState},
        pin::Pin,
        ptr::NonNull,
        task::{Context, Poll},
    };

    use futures_core::stream::{FusedStream, Stream};
    use pin_project::pin_project;

    use crate::future::ResumeTy;

    /// Wrap a generator in a stream.
    ///
    /// This function returns a `GenStream` underneath, but hides it in `impl Trait` to give
    /// better error messages (`impl Stream` rather than `GenStream<[closure.....]>`).
    #[doc(hidden)]
    pub fn from_generator<G, T, E>(
        gen: G,
    ) -> impl Stream<Item = Result<T, E>> + FusedStream<Item = Result<T, E>>
    where
        G: Generator<ResumeTy, Yield = Poll<T>, Return = Result<(), E>>,
    {
        GenTryStream(Some(gen))
    }

    #[pin_project]
    pub(crate) struct GenTryStream<G>(#[pin] Option<G>);

    impl<G, T, E> Stream for GenTryStream<G>
    where
        G: Generator<ResumeTy, Yield = Poll<T>, Return = Result<(), E>>,
    {
        type Item = Result<T, E>;

        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            let mut this = self.project();
            if let Some(gen) = this.0.as_mut().as_pin_mut() {
                let res = match gen.resume(ResumeTy(NonNull::from(cx).cast::<Context<'static>>())) {
                    GeneratorState::Yielded(x) => x.map(|x| Some(Ok(x))),
                    GeneratorState::Complete(Err(e)) => Poll::Ready(Some(Err(e))),
                    GeneratorState::Complete(Ok(())) => Poll::Ready(None),
                };
                if let Poll::Ready(Some(Err(_)) | None) = &res {
                    this.0.set(None);
                }
                res
            } else {
                Poll::Ready(None)
            }
        }
    }

    impl<G, T, E> FusedStream for GenTryStream<G>
    where
        G: Generator<ResumeTy, Yield = Poll<T>, Return = Result<(), E>>,
    {
        fn is_terminated(&self) -> bool {
            self.0.is_none()
        }
    }
}

// Not public API.
#[doc(hidden)]
pub mod __private {
    #[doc(hidden)]
    pub use core::{
        marker::Send,
        option::Option::{None, Some},
        pin::Pin,
        result::Result::{self, Ok},
        task::Poll,
    };

    #[doc(hidden)]
    pub mod future {
        #[doc(hidden)]
        pub use core::future::Future;

        #[doc(hidden)]
        #[allow(unreachable_pub)] // https://github.com/rust-lang/rust/issues/102352
        pub use crate::future::{from_generator, get_context, ResumeTy};
    }

    #[doc(hidden)]
    pub mod stream {
        #[doc(hidden)]
        pub use futures_core::stream::Stream;

        #[doc(hidden)]
        pub use crate::stream::{from_generator, next};
    }

    #[doc(hidden)]
    pub mod try_stream {
        #[doc(hidden)]
        pub use crate::try_stream::from_generator;
    }
}

#[allow(clippy::wildcard_imports)]
#[cfg(test)]
mod tests {
    use core::marker::PhantomPinned;

    use static_assertions::{
        assert_impl_all as assert_impl, assert_not_impl_all as assert_not_impl,
    };

    use crate::*;

    assert_impl!(future::GenFuture<()>: Send);
    assert_not_impl!(future::GenFuture<*const ()>: Send);
    assert_impl!(future::GenFuture<()>: Sync);
    assert_not_impl!(future::GenFuture<*const ()>: Sync);
    assert_impl!(future::GenFuture<()>: Unpin);
    assert_not_impl!(future::GenFuture<PhantomPinned>: Unpin);

    assert_impl!(stream::GenStream<()>: Send);
    assert_not_impl!(stream::GenStream<*const ()>: Send);
    assert_impl!(stream::GenStream<()>: Sync);
    assert_not_impl!(stream::GenStream<*const ()>: Sync);
    assert_impl!(stream::GenStream<()>: Unpin);
    assert_not_impl!(stream::GenStream<PhantomPinned>: Unpin);

    assert_impl!(stream::Next<'_, ()>: Send);
    assert_not_impl!(stream::Next<'_, *const ()>: Send);
    assert_impl!(stream::Next<'_, ()>: Sync);
    assert_not_impl!(stream::Next<'_, *const ()>: Sync);
    assert_impl!(stream::Next<'_, PhantomPinned>: Unpin);

    assert_impl!(try_stream::GenTryStream<()>: Send);
    assert_not_impl!(try_stream::GenTryStream<*const ()>: Send);
    assert_impl!(try_stream::GenTryStream<()>: Sync);
    assert_not_impl!(try_stream::GenTryStream<*const ()>: Sync);
    assert_impl!(try_stream::GenTryStream<()>: Unpin);
    assert_not_impl!(try_stream::GenTryStream<PhantomPinned>: Unpin);
}
