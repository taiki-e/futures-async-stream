use core::{
    marker::PhantomData,
    ops::{Generator, GeneratorState},
    pin::Pin,
    task::{Context, Poll},
};
use futures_core::stream::{FusedStream, Stream};
use pin_project::pin_project;
use std::future;

// =================================================================================================
// GenTryStream

/// Wrap a generator in a stream.
///
/// This function returns a `GenTryStream` underneath, but hides it in `impl Trait` to give
/// better error messages (`impl Stream` rather than `GenTryStream<[closure.....]>`).
#[doc(hidden)]
pub fn from_generator<G, T, E>(
    gen: G,
) -> impl Stream<Item = Result<T, E>> + FusedStream<Item = Result<T, E>>
where
    G: Generator<Yield = Poll<T>, Return = Result<(), E>>,
{
    GenTryStream { gen, done: false, _phantom: PhantomData }
}

/// A wrapper around generators used to implement `Stream` for `async`/`await` code.
#[pin_project]
#[derive(Debug)]
struct GenTryStream<G, T, E> {
    #[pin]
    gen: G,
    done: bool,
    _phantom: PhantomData<(T, E)>,
}

impl<G, T, E> Stream for GenTryStream<G, T, E>
where
    G: Generator<Yield = Poll<T>, Return = Result<(), E>>,
{
    type Item = Result<T, E>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.done {
            return Poll::Ready(None);
        }

        let mut this = self.project();
        let res = future::set_task_context(cx, || match this.gen.as_mut().resume() {
            GeneratorState::Yielded(x) => x.map(|x| Some(Ok(x))),
            GeneratorState::Complete(Err(e)) => Poll::Ready(Some(Err(e))),
            GeneratorState::Complete(Ok(())) => Poll::Ready(None),
        });
        if let Poll::Ready(Some(Err(_))) | Poll::Ready(None) = &res {
            *this.done = true;
        }
        res
    }
}

impl<G, T, E> FusedStream for GenTryStream<G, T, E>
where
    G: Generator<Yield = Poll<T>, Return = Result<(), E>>,
{
    fn is_terminated(&self) -> bool {
        self.done
    }
}
