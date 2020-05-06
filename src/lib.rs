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
//! `value` has the `Item` type of the stream passed in. Note that async for loops can only be used inside of `async` functions, closures, blocks, `#[stream]` functions and `stream_block!` macros.
//!
//! ## \#\[stream\]
//!
//! Creates streams via generators.
//!
//! This is a reimplement of [futures-await]'s `#[stream]` for futures 0.3 and is an experimental implementation of [the idea listed as the next step of async/await](https://github.com/rust-lang/rfcs/blob/master/text/2394-async_await.md#generators-and-streams).
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
//! `#[stream]` must have an item type specified via `item = some::Path` and the values output from the stream must be yielded via the `yield` expression.
//!
//! ## stream_block!
//!
//! You can create a stream directly as an expression using an `stream_block!` macro:
//!
//! ```rust
//! #![feature(generators, proc_macro_hygiene)]
//!
//! use futures::stream::Stream;
//! use futures_async_stream::stream_block;
//!
//! fn foo() -> impl Stream<Item = i32> {
//!     stream_block! {
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
//!
//! use futures::stream::Stream;
//! use futures_async_stream::stream;
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
//!     #[stream(boxed, item = u32)]
//!     async fn method(&mut self) {
//!         while self.0 < u32::max_value() {
//!             self.0 += 1;
//!             yield self.0;
//!         }
//!     }
//! }
//! ```
//!
//! ## \#\[try_stream\]
//!
//! `?` operator can be used with the `#[try_stream]`. The `Item` of the returned stream is `Result` with `Ok` being the value yielded and `Err` the error type returned by `?` operator or `return Err(...)`.
//!
//! ```rust
//! #![feature(generators)]
//!
//! use futures::stream::Stream;
//! use futures_async_stream::try_stream;
//!
//! #[try_stream(ok = i32, error = Box<dyn std::error::Error + Send + Sync>)]
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
//! ### \#\[stream\]
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
#![doc(html_root_url = "https://docs.rs/futures-async-stream/0.1.5")]
#![doc(test(
    no_crate_inject,
    attr(deny(warnings, rust_2018_idioms, single_use_lifetimes), allow(dead_code))
))]
#![warn(missing_docs)]
#![warn(rust_2018_idioms, single_use_lifetimes, unreachable_pub)]
#![warn(clippy::all, clippy::default_trait_access)]
#![feature(generator_trait, generators)]

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
    use futures_core::ready;
    use pin_project::{pin_project, project};

    // Refs: https://github.com/rust-lang/rust/blob/2454a68cfbb63aa7b8e09fe05114d5f98b2f9740/src/libcore/future/mod.rs

    /// This type is needed because:
    ///
    /// a) Generators cannot implement `for<'a, 'b> Generator<&'a mut Context<'b>>`, so we need to pass
    ///    a raw pointer (see https://github.com/rust-lang/rust/issues/68923).
    /// b) Raw pointers and `NonNull` aren't `Send` or `Sync`, so that would make every single future
    ///    non-Send/Sync as well, and we don't want that.
    ///
    /// It also simplifies the lowering of `.await`.
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
        #[pin_project]
        struct GenFuture<G>(#[pin] G);

        impl<G> Future for GenFuture<G>
        where
            G: Generator<ResumeTy, Yield = ()>,
        {
            type Output = G::Return;

            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                let this = self.project();
                match this.0.resume(ResumeTy(NonNull::from(cx).cast::<Context<'static>>())) {
                    GeneratorState::Yielded(()) => Poll::Pending,
                    GeneratorState::Complete(x) => Poll::Ready(x),
                }
            }
        }

        GenFuture(gen)
    }

    #[doc(hidden)]
    pub unsafe fn get_context<'a, 'b>(cx: ResumeTy) -> &'a mut Context<'b> {
        &mut *cx.0.as_ptr().cast()
    }

    /// A future that may have completed.
    ///
    /// This is created by the [`maybe_done()`] function.
    #[doc(hidden)]
    #[pin_project(Replace)]
    pub enum MaybeDone<F: Future> {
        /// A not-yet-completed future
        Future(#[pin] F),
        /// The output of the completed future
        Done(F::Output),
        /// The empty variant after the result of a [`MaybeDone`] has been
        /// taken using the [`take_output`](MaybeDone::take_output) method.
        Gone,
    }

    /// Wraps a future into a `MaybeDone`
    #[doc(hidden)]
    pub fn maybe_done<F: Future>(future: F) -> MaybeDone<F> {
        MaybeDone::Future(future)
    }

    impl<F: Future> MaybeDone<F> {
        /// Attempt to take the output of a `MaybeDone` without driving it
        /// towards completion.
        pub fn take_output(self: Pin<&mut Self>) -> Option<F::Output> {
            match &*self {
                MaybeDone::Done(_) => {}
                MaybeDone::Future(_) | MaybeDone::Gone => return None,
            }
            if let __MaybeDoneProjectionOwned::Done(output) = self.project_replace(MaybeDone::Gone)
            {
                Some(output)
            } else {
                unreachable!()
            }
        }
    }

    impl<F: Future> Future for MaybeDone<F> {
        type Output = ();

        #[project]
        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            #[project]
            match self.as_mut().project() {
                MaybeDone::Future(f) => {
                    let res = ready!(f.poll(cx));
                    self.set(MaybeDone::Done(res));
                }
                MaybeDone::Done(_) => {}
                MaybeDone::Gone => panic!("MaybeDone polled after value taken"),
            }
            Poll::Ready(())
        }
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
        #[pin_project]
        struct GenStream<G>(#[pin] G);

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

        GenStream(gen)
    }

    // This is equivalent to the `futures::stream::StreamExt::next` method.
    // But we want to make this crate dependency as small as possible, so we define our `next` function.
    #[doc(hidden)]
    pub fn next<S>(stream: &mut S) -> impl Future<Output = Option<S::Item>> + '_
    where
        S: Stream + Unpin,
    {
        struct Next<'a, S>(&'a mut S);

        impl<S> Future for Next<'_, S>
        where
            S: Stream + Unpin,
        {
            type Output = Option<S::Item>;

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                Pin::new(&mut self.0).poll_next(cx)
            }
        }

        Next(stream)
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
        #[pin_project]
        struct GenTryStream<G>(#[pin] Option<G>);

        impl<G, T, E> Stream for GenTryStream<G>
        where
            G: Generator<ResumeTy, Yield = Poll<T>, Return = Result<(), E>>,
        {
            type Item = Result<T, E>;

            fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
                let mut this = self.project();
                if let Some(gen) = this.0.as_mut().as_pin_mut() {
                    let res =
                        match gen.resume(ResumeTy(NonNull::from(cx).cast::<Context<'static>>())) {
                            GeneratorState::Yielded(x) => x.map(|x| Some(Ok(x))),
                            GeneratorState::Complete(Err(e)) => Poll::Ready(Some(Err(e))),
                            GeneratorState::Complete(Ok(())) => Poll::Ready(None),
                        };
                    if let Poll::Ready(Some(Err(_))) | Poll::Ready(None) = &res {
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

        GenTryStream(Some(gen))
    }
}

// Not public API.
#[doc(hidden)]
pub mod sink {
    use core::{
        ops::{Generator, GeneratorState},
        pin::Pin,
        ptr::NonNull,
        task::{Context, Poll},
    };
    use futures_sink::Sink;
    use pin_project::pin_project;

    use crate::future::ResumeTy;

    #[doc(hidden)]
    pub enum Arg<T> {
        StartSend(T),
        Flush(ResumeTy),
        Close(ResumeTy),
    }

    #[doc(hidden)]
    pub enum Res {
        Idle,
        Pending,
        Accepted,
    }

    /// This `Sink` is complete and no longer accepting items
    #[derive(Debug, PartialEq, Eq)]
    pub struct Complete;

    /// Wrap a generator in a sink.
    ///
    /// This function returns a `GenStream` underneath, but hides it in `impl Trait` to give
    /// better error messages (`impl Stream` rather than `GenStream<[closure.....]>`).
    #[doc(hidden)]
    pub fn from_generator<G, T>(gen: G) -> impl Sink<T, Error = Complete>
    where
        G: Generator<Arg<T>, Yield = Res, Return = ()>,
    {
        GenSink { gen }
    }

    #[pin_project]
    struct GenSink<G> {
        #[pin]
        gen: G,
    }

    impl<T, G> Sink<T> for GenSink<G>
    where
        G: Generator<Arg<T>, Yield = Res, Return = ()>,
    {
        type Error = Complete;

        fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            self.poll_flush(cx)
        }

        fn start_send(self: Pin<&mut Self>, item: T) -> Result<(), Self::Error> {
            match self.project().gen.resume(Arg::StartSend(item)) {
                GeneratorState::Yielded(Res::Idle) => panic!("sink idle after start send"),
                GeneratorState::Yielded(Res::Pending) => panic!("sink rejected send"),
                GeneratorState::Yielded(Res::Accepted) => Ok(()),
                GeneratorState::Complete(()) => Err(Complete),
            }
        }

        fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            match self
                .project()
                .gen
                .resume(Arg::Flush(ResumeTy(NonNull::from(cx).cast::<Context<'static>>())))
            {
                GeneratorState::Yielded(Res::Idle) => Poll::Ready(Ok(())),
                GeneratorState::Yielded(Res::Pending) => Poll::Pending,
                GeneratorState::Yielded(Res::Accepted) => panic!("sink accepted during flush"),
                GeneratorState::Complete(()) => Poll::Ready(Err(Complete)),
            }
        }

        fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            match self
                .project()
                .gen
                .resume(Arg::Close(ResumeTy(NonNull::from(cx).cast::<Context<'static>>())))
            {
                GeneratorState::Yielded(Res::Idle) => panic!("should have returned"),
                GeneratorState::Yielded(Res::Pending) => Poll::Pending,
                GeneratorState::Yielded(Res::Accepted) => panic!("sink accepted during close"),
                GeneratorState::Complete(()) => Poll::Ready(Err(Complete)),
            }
        }
    }
}

// Not public API.
#[doc(hidden)]
pub mod try_sink {
    use core::{
        ops::{Generator, GeneratorState},
        pin::Pin,
        ptr::NonNull,
        task::{Context, Poll},
    };
    use futures_sink::Sink;
    use pin_project::pin_project;

    use crate::future::ResumeTy;

    #[doc(hidden)]
    pub enum Arg<T> {
        StartSend(T),
        Flush(ResumeTy),
        Close(ResumeTy),
    }

    #[doc(hidden)]
    pub enum Res {
        Idle,
        Pending,
        Accepted,
    }

    /// Wrap a generator in a sink.
    ///
    /// This function returns a `GenSink` underneath, but hides it in `impl Trait` to give
    /// better error messages (`impl Stream` rather than `GenSink<[closure.....]>`).
    #[doc(hidden)]
    pub fn from_generator<G, T, E>(gen: G) -> impl Sink<T, Error = E>
    where
        G: Generator<Arg<T>, Yield = Res, Return = Result<(), E>>,
    {
        GenSink { gen }
    }

    #[pin_project]
    struct GenSink<G> {
        #[pin]
        gen: G,
    }

    impl<T, E, G> Sink<T> for GenSink<G>
    where
        G: Generator<Arg<T>, Yield = Res, Return = Result<(), E>>,
    {
        type Error = E;

        fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            self.poll_flush(cx)
        }

        fn start_send(self: Pin<&mut Self>, item: T) -> Result<(), Self::Error> {
            match self.project().gen.resume(Arg::StartSend(item)) {
                GeneratorState::Yielded(Res::Idle) => panic!("sink idle after start send"),
                GeneratorState::Yielded(Res::Pending) => panic!("sink rejected send"),
                GeneratorState::Yielded(Res::Accepted) => Ok(()),
                GeneratorState::Complete(Ok(())) => panic!("sink unexpectedly closed"),
                GeneratorState::Complete(Err(e)) => Err(e),
            }
        }

        fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            match self
                .project()
                .gen
                .resume(Arg::Flush(ResumeTy(NonNull::from(cx).cast::<Context<'static>>())))
            {
                GeneratorState::Yielded(Res::Idle) => Poll::Ready(Ok(())),
                GeneratorState::Yielded(Res::Pending) => Poll::Pending,
                GeneratorState::Yielded(Res::Accepted) => panic!("sink accepted during flush"),
                GeneratorState::Complete(Ok(())) => Poll::Ready(Ok(())),
                GeneratorState::Complete(Err(e)) => Poll::Ready(Err(e)),
            }
        }

        fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            match self
                .project()
                .gen
                .resume(Arg::Close(ResumeTy(NonNull::from(cx).cast::<Context<'static>>())))
            {
                GeneratorState::Yielded(Res::Idle) => panic!("should have returned"),
                GeneratorState::Yielded(Res::Pending) => Poll::Pending,
                GeneratorState::Yielded(Res::Accepted) => panic!("sink accepted during close"),
                GeneratorState::Complete(Ok(())) => Poll::Ready(Ok(())),
                GeneratorState::Complete(Err(e)) => Poll::Ready(Err(e)),
            }
        }
    }

    macro_rules! await_in_sink {
        ($arg:ident, $e:expr) => {{}};
    }

    macro_rules! await_item {
        ($arg:ident) => {{}};
    }

    extern crate std;
    use std::string::String;

    async fn bar(s: String) -> Result<(), String> {
        if s == "bang" { Err(s) } else { Ok(()) }
    }

    fn foo() -> impl Sink<String, Error = String> {
        from_generator(static move |mut arg: Arg<String>| -> Result<(), String> {
            while let Some(item) = {
                let mut item = None;
                loop {
                    match item {
                        Some(item) => break item,
                        None => {
                            arg = match arg {
                                Arg::StartSend(i) => {
                                    item = Some(Some(i));
                                    yield Res::Accepted
                                }
                                Arg::Flush(cx) => yield Res::Idle,
                                Arg::Close(cx) => {
                                    item = Some(None);
                                    Arg::Close(cx)
                                }
                            };
                        }
                    }
                }
            } {
                // await_in_sink!(arg, bar(item))
                let e = bar(item);
                let _i = {
                    use crate::future::MaybeDone;
                    let mut future = MaybeDone::Future(e);
                    let mut future = unsafe { Pin::new_unchecked(&mut future) };
                    loop {
                        if let Some(e) = future.as_mut().take_output() {
                            break e;
                        }
                        arg = match arg {
                            Arg::StartSend(item) => yield Res::Pending,
                            Arg::Flush(cx) | Arg::Close(cx) => {
                                if let Poll::Ready(()) = unsafe {
                                    core::future::Future::poll(
                                        future.as_mut(),
                                        crate::future::get_context(cx),
                                    )
                                } {}
                                yield Res::Pending
                            }
                        }
                    }
                };
            }

            Ok(())
        })
    }

    // fn test() {
    //     let sink = foo();
    // }
}

// Not public API.
#[doc(hidden)]
pub mod __reexport {
    #[doc(hidden)]
    pub use core::{marker, option, pin, result, task};

    #[doc(hidden)]
    pub mod future {
        #[doc(hidden)]
        pub use core::future::Future;

        #[doc(hidden)]
        pub use crate::future::*;
    }

    #[doc(hidden)]
    pub mod stream {
        #[doc(hidden)]
        pub use futures_core::stream::Stream;

        #[doc(hidden)]
        pub use crate::stream::*;
    }

    #[doc(hidden)]
    pub mod try_stream {
        #[doc(hidden)]
        pub use crate::try_stream::*;
    }
}
