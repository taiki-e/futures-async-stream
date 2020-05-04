//! Async stream API experiment that may be introduced as a language feature in the future.
//!
//! This crate provides useful features for streams, using `async_await` and unstable `generators`.
//!
//! ## \#\[for_await\]
//!
//! Processes streams using a for loop.
//!
//! This is a reimplement of [futures-await]'s `#[async]` for loops for futures 0.3 and is an experimental implementation of [the idea listed as the next step of async/await](https://github.com/rust-lang/rfcs/blob/master/text/2394-async_await.md#for-await-and-processing-streams).
//!
//! ```rust
//! #![feature(stmt_expr_attributes, proc_macro_hygiene)]
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
//! ## \#\[async_stream\]
//!
//! Creates streams via generators.
//!
//! This is a reimplement of [futures-await]'s `#[async_stream]` for futures 0.3 and is an experimental implementation of [the idea listed as the next step of async/await](https://github.com/rust-lang/rfcs/blob/master/text/2394-async_await.md#generators-and-streams).
//!
//! ```rust
//! #![feature(generators)]
//! use futures::stream::Stream;
//! use futures_async_stream::async_stream;
//!
//! // Returns a stream of i32
//! #[async_stream(item = i32)]
//! async fn foo(stream: impl Stream<Item = String>) {
//!     // `for_await` is built into `async_stream`. If you use `for_await` only in `async_stream`, there is no need to import `for_await`.
//!     #[for_await]
//!     for x in stream {
//!         yield x.parse().unwrap();
//!     }
//! }
//! ```
//!
//! `#[async_stream]` must have an item type specified via `item = some::Path` and the values output from the stream must be yielded via the `yield` expression.
//!
//! ## async_stream_block!
//!
//! You can create a stream directly as an expression using an `async_stream_block!` macro:
//!
//! ```rust
//! #![feature(generators, proc_macro_hygiene)]
//! use futures::stream::Stream;
//! use futures_async_stream::async_stream_block;
//!
//! fn foo() -> impl Stream<Item = i32> {
//!     async_stream_block! {
//!         for i in 0..10 {
//!             yield i;
//!         }
//!     }
//! }
//! ```
//!
//! ## Using async stream functions in traits
//!
//! You can use async stream functions in traits by passing `boxed` or `boxed_local` as an argument.
//!
//! ```rust
//! #![feature(generators)]
//! use futures_async_stream::async_stream;
//!
//! trait Foo {
//!     #[async_stream(boxed, item = u32)]
//!     async fn method(&mut self);
//! }
//!
//! struct Bar(u32);
//!
//! impl Foo for Bar {
//!     #[async_stream(boxed, item = u32)]
//!     async fn method(&mut self) {
//!         while self.0 < u32::max_value() {
//!             self.0 += 1;
//!             yield self.0;
//!         }
//!     }
//! }
//! ```
//!
//! A async stream function that received a `boxed` argument is converted to a function that returns `Pin<Box<dyn Stream<Item = item> + Send + 'lifetime>>`.
//! If you passed `boxed_local` instead of `boxed`, async stream function returns a non-threadsafe stream (`Pin<Box<dyn Stream<Item = item> + 'lifetime>>`).
//!
//! ```rust
//! #![feature(generators)]
//! use futures::stream::Stream;
//! use futures_async_stream::async_stream;
//! use std::pin::Pin;
//!
//! // The trait itself can be defined without unstable features.
//! trait Foo {
//!     fn method(&mut self) -> Pin<Box<dyn Stream<Item = u32> + Send + '_>>;
//! }
//!
//! struct Bar(u32);
//!
//! impl Foo for Bar {
//!     #[async_stream(boxed, item = u32)]
//!     async fn method(&mut self) {
//!         while self.0 < u32::max_value() {
//!             self.0 += 1;
//!             yield self.0;
//!         }
//!     }
//! }
//! ```
//!
//! ## \#\[async_try_stream\]
//!
//! `?` operator can be used with the `#[async_try_stream]`. The `Item` of the returned stream is `Result` with `Ok` being the value yielded and `Err` the error type returned by `?` operator or `return Err(...)`.
//!
//! ```rust
//! #![feature(generators)]
//! use futures::stream::Stream;
//! use futures_async_stream::async_try_stream;
//!
//! #[async_try_stream(ok = i32, error = Box<dyn std::error::Error + Send + Sync>)]
//! async fn foo(stream: impl Stream<Item = String>) {
//!     #[for_await]
//!     for x in stream {
//!         yield x.parse()?;
//!     }
//! }
//! ```
//!
//! ## How to write the equivalent code without this API?
//!
//! ### \#\[for_await\]
//!
//! You can write this by combining `while let` loop, `.await`, `pin_mut` macro, and `StreamExt::next()` method:
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
//! ### \#\[async_stream\]
//!
//! You can write this by manually implementing the combinator:
//!
//! ```rust
//! use futures::{
//!     ready,
//!     stream::Stream,
//!     task::{Context, Poll},
//! };
//! use pin_project::pin_project;
//! use std::pin::Pin;
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
#![doc(html_root_url = "https://docs.rs/futures-async-stream/0.1.4")]
#![doc(test(
    no_crate_inject,
    attr(deny(warnings, rust_2018_idioms, single_use_lifetimes), allow(dead_code))
))]
#![warn(missing_docs)]
#![warn(rust_2018_idioms, single_use_lifetimes, unreachable_pub)]
#![warn(clippy::all)]
#![feature(generator_trait)]

#[doc(inline)]
pub use futures_async_stream_macro::for_await;

#[doc(inline)]
pub use futures_async_stream_macro::async_stream;

#[doc(inline)]
pub use futures_async_stream_macro::async_stream_block;

#[doc(inline)]
pub use futures_async_stream_macro::async_try_stream;

#[doc(inline)]
pub use futures_async_stream_macro::async_try_stream_block;

// Not public API.
#[doc(hidden)]
pub mod future {
    use core::{
        ops::{Generator, GeneratorState},
        pin::Pin,
        ptr::NonNull,
        task::{Context, Poll},
    };
    use pin_project::pin_project;

    #[doc(hidden)]
    pub use core::future::Future;

    /// This type is needed because:
    ///
    /// a) Generators cannot implement `for<'a, 'b> Generator<&'a mut Context<'b>>`, so we need to pass
    ///    a raw pointer.
    /// b) Raw pointers and `NonNull` aren't `Send` or `Sync`, so that would make every single future
    ///    non-Send/Sync as well, and we don't want that.
    ///
    /// It also simplifies the HIR lowering of `.await`.
    #[doc(hidden)]
    #[derive(Debug, Copy, Clone)]
    pub struct ResumeTy(pub(crate) NonNull<Context<'static>>);

    unsafe impl Send for ResumeTy {}

    unsafe impl Sync for ResumeTy {}

    /// Wrap a generator in a future.
    ///
    /// This function returns a `GenFuture` underneath, but hides it in `impl Trait` to give
    /// better error messages (`impl Future` rather than `GenFuture<[closure.....]>`).
    #[doc(hidden)]
    pub fn from_generator<G>(gen: G) -> impl Future<Output = G::Return>
    where
        G: Generator<ResumeTy, Yield = ()>,
    {
        GenFuture { gen }
    }

    /// A wrapper around generators used to implement `Future` for `async`/`await` code.
    #[pin_project]
    struct GenFuture<G> {
        #[pin]
        gen: G,
    }

    impl<G> Future for GenFuture<G>
    where
        G: Generator<ResumeTy, Yield = ()>,
    {
        type Output = G::Return;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let this = self.project();
            match this.gen.resume(ResumeTy(NonNull::from(cx).cast::<Context<'static>>())) {
                GeneratorState::Yielded(()) => Poll::Pending,
                GeneratorState::Complete(x) => Poll::Ready(x),
            }
        }
    }

    #[doc(hidden)]
    pub unsafe fn get_context<'a, 'b>(cx: ResumeTy) -> &'a mut Context<'b> {
        &mut *cx.0.as_ptr().cast()
    }
}

// Not public API.
#[doc(hidden)]
pub mod stream {
    use core::{
        future::Future,
        marker::PhantomData,
        ops::{Generator, GeneratorState},
        pin::Pin,
        ptr::NonNull,
        task::{Context, Poll},
    };
    use pin_project::pin_project;

    use super::future::ResumeTy;

    #[doc(hidden)]
    pub use futures_core::stream::Stream;

    /// Wrap a generator in a stream.
    ///
    /// This function returns a `GenStream` underneath, but hides it in `impl Trait` to give
    /// better error messages (`impl Stream` rather than `GenStream<[closure.....]>`).
    #[doc(hidden)]
    pub fn from_generator<G, T>(gen: G) -> impl Stream<Item = T>
    where
        G: Generator<ResumeTy, Yield = Poll<T>, Return = ()>,
    {
        GenStream { gen, _phantom: PhantomData }
    }

    /// A wrapper around generators used to implement `Stream` for `async`/`await` code.
    #[pin_project]
    struct GenStream<G, T> {
        #[pin]
        gen: G,
        _phantom: PhantomData<T>,
    }

    impl<G, T> Stream for GenStream<G, T>
    where
        G: Generator<ResumeTy, Yield = Poll<T>, Return = ()>,
    {
        type Item = T;

        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            let this = self.project();
            match this.gen.resume(ResumeTy(NonNull::from(cx).cast::<Context<'static>>())) {
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
        Next { stream }
    }

    struct Next<'a, S> {
        stream: &'a mut S,
    }

    impl<S> Future for Next<'_, S>
    where
        S: Stream + Unpin,
    {
        type Output = Option<S::Item>;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            Pin::new(&mut self.stream).poll_next(cx)
        }
    }
}

// Not public API.
#[doc(hidden)]
pub mod try_stream {
    use core::{
        marker::PhantomData,
        ops::{Generator, GeneratorState},
        pin::Pin,
        ptr::NonNull,
        task::{Context, Poll},
    };
    use futures_core::stream::{FusedStream, Stream};
    use pin_project::pin_project;

    use super::future::ResumeTy;

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
        GenTryStream { gen, done: false, _phantom: PhantomData }
    }

    /// A wrapper around generators used to implement `Stream` for `async`/`await` code.
    #[pin_project]
    struct GenTryStream<G, T, E> {
        #[pin]
        gen: G,
        done: bool,
        _phantom: PhantomData<(T, E)>,
    }

    impl<G, T, E> Stream for GenTryStream<G, T, E>
    where
        G: Generator<ResumeTy, Yield = Poll<T>, Return = Result<(), E>>,
    {
        type Item = Result<T, E>;

        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            if self.done {
                return Poll::Ready(None);
            }

            let this = self.project();
            let res = match this.gen.resume(ResumeTy(NonNull::from(cx).cast::<Context<'static>>()))
            {
                GeneratorState::Yielded(x) => x.map(|x| Some(Ok(x))),
                GeneratorState::Complete(Err(e)) => Poll::Ready(Some(Err(e))),
                GeneratorState::Complete(Ok(())) => Poll::Ready(None),
            };
            if let Poll::Ready(Some(Err(_))) | Poll::Ready(None) = &res {
                *this.done = true;
            }
            res
        }
    }

    impl<G, T, E> FusedStream for GenTryStream<G, T, E>
    where
        G: Generator<ResumeTy, Yield = Poll<T>, Return = Result<(), E>>,
    {
        fn is_terminated(&self) -> bool {
            self.done
        }
    }
}

// Not public API.
#[doc(hidden)]
pub mod __reexport {
    #[doc(hidden)]
    pub use core::{marker, option, pin, result, task};
}
