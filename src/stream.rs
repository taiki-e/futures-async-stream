use pin_project::pin_project;
use std::{
    future::{self, Future},
    marker::PhantomData,
    ops::{Generator, GeneratorState},
    pin::Pin,
    task::{Context, Poll},
};

pub use futures_core::stream::Stream;

// =================================================================================================
// GenStream

/// Wrap a generator in a stream.
///
/// This function returns a `GenStream` underneath, but hides it in `impl Trait` to give
/// better error messages (`impl Stream` rather than `GenStream<[closure.....]>`).
#[doc(hidden)]
pub fn from_generator<U, T>(x: T) -> impl Stream<Item = U>
where
    T: Generator<Yield = Poll<U>, Return = ()>,
{
    GenStream { gen: x, _phantom: PhantomData }
}

/// A wrapper around generators used to implement `Stream` for `async`/`await` code.
#[pin_project]
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
struct GenStream<U, T> {
    #[pin]
    gen: T,
    _phantom: PhantomData<U>,
}

impl<U, T> Stream for GenStream<U, T>
where
    T: Generator<Yield = Poll<U>, Return = ()>,
{
    type Item = U;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        future::set_task_context(cx, || match this.gen.resume() {
            GeneratorState::Yielded(x) => x.map(Some),
            GeneratorState::Complete(()) => Poll::Ready(None),
        })
    }
}

/// Polls a stream in the current thread-local task waker.
#[doc(hidden)]
pub fn poll_next_with_tls_context<S>(s: Pin<&mut S>) -> Poll<Option<S::Item>>
where
    S: Stream,
{
    future::get_task_context(|cx| S::poll_next(s, cx))
}

// This is equivalent to the `std::future::poll_with_tls_context` method.
// The `.await` called in `async_stream` needs to be adjusted to yield `Poll`,
// but for this purpose, we don't want the user to use `#![feature(gen_future)]`.
/// Polls a future in the current thread-local task waker.
#[doc(hidden)]
pub fn poll_with_tls_context<F>(f: Pin<&mut F>) -> Poll<F::Output>
where
    F: Future,
{
    future::get_task_context(|cx| F::poll(f, cx))
}

// =================================================================================================
// Next

// This is equivalent to the `futures::StreamExt::next` method.
// But we want to make this crate dependency as small as possible, so we define our `next` function.
#[doc(hidden)]
pub fn next<S>(stream: &mut S) -> impl Future<Output = Option<S::Item>> + '_
where
    S: Stream + Unpin,
{
    Next { stream }
}

#[derive(Debug)]
struct Next<'a, S> {
    stream: &'a mut S,
}

impl<S: Unpin> Unpin for Next<'_, S> {}

impl<S: Stream + Unpin> Future for Next<'_, S> {
    type Output = Option<S::Item>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.stream).poll_next(cx)
    }
}
