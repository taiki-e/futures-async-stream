# futures-async-stream

[![crates.io](https://img.shields.io/crates/v/futures-async-stream?style=flat-square&logo=rust)](https://crates.io/crates/futures-async-stream)
[![docs.rs](https://img.shields.io/badge/docs.rs-futures--async--stream-blue?style=flat-square&logo=docs.rs)](https://docs.rs/futures-async-stream)
[![license](https://img.shields.io/badge/license-Apache--2.0_OR_MIT-blue?style=flat-square)](#license)
[![github actions](https://img.shields.io/github/actions/workflow/status/taiki-e/futures-async-stream/ci.yml?branch=main&style=flat-square&logo=github)](https://github.com/taiki-e/futures-async-stream/actions)

<!-- tidy:crate-doc:start -->
Async stream for Rust and the futures crate.

This crate provides useful features for streams, using `async_await` and
unstable [`coroutines`](https://github.com/rust-lang/rust/issues/43122).

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
futures-async-stream = "0.2"
futures = "0.3"
```

*Compiler support: requires rustc nightly-2023-10-29+*

## `#[for_await]`

Processes streams using a for loop.

This is a reimplement of [futures-await]'s `#[async]` for loops for
futures 0.3 and is an experimental implementation of [the idea listed as the
next step of async/await](https://github.com/rust-lang/rfcs/blob/HEAD/text/2394-async_await.md#for-await-and-processing-streams).

```rust
#![feature(proc_macro_hygiene, stmt_expr_attributes)]

use futures::stream::Stream;
use futures_async_stream::for_await;

async fn collect(stream: impl Stream<Item = i32>) -> Vec<i32> {
    let mut vec = vec![];
    #[for_await]
    for value in stream {
        vec.push(value);
    }
    vec
}
```

`value` has the `Item` type of the stream passed in. Note that async for
loops can only be used inside of `async` functions, closures, blocks,
`#[stream]` functions and `stream_block!` macros.

## `#[stream]`

Creates streams via coroutines.

This is a reimplement of [futures-await]'s `#[stream]` for futures 0.3 and
is an experimental implementation of [the idea listed as the next step of
async/await](https://github.com/rust-lang/rfcs/blob/HEAD/text/2394-async_await.md#generators-and-streams).

```rust
#![feature(coroutines)]

use futures::stream::Stream;
use futures_async_stream::stream;

// Returns a stream of i32
#[stream(item = i32)]
async fn foo(stream: impl Stream<Item = String>) {
    // `for_await` is built into `stream`. If you use `for_await` only in `stream`, there is no need to import `for_await`.
    #[for_await]
    for x in stream {
        yield x.parse().unwrap();
    }
}
```

To early exit from a `#[stream]` function or block, use `return`.

`#[stream]` on async fn must have an item type specified via
`item = some::Path` and the values output from the stream must be yielded
via the `yield` expression.

`#[stream]` can also be used on async blocks:

```rust
#![feature(coroutines, proc_macro_hygiene, stmt_expr_attributes)]

use futures::stream::Stream;
use futures_async_stream::stream;

fn foo() -> impl Stream<Item = i32> {
    #[stream]
    async move {
        for i in 0..10 {
            yield i;
        }
    }
}
```

Note that `#[stream]` on async block does not require the `item` argument,
but it may require additional type annotations.

## Using async stream functions in traits

You can use async stream functions in traits by passing `boxed` or
`boxed_local` as an argument.

```rust
#![feature(coroutines)]

use futures_async_stream::stream;

trait Foo {
    #[stream(boxed, item = u32)]
    async fn method(&mut self);
}

struct Bar(u32);

impl Foo for Bar {
    #[stream(boxed, item = u32)]
    async fn method(&mut self) {
        while self.0 < u32::MAX {
            self.0 += 1;
            yield self.0;
        }
    }
}
```

A async stream function that received a `boxed` argument is converted to a
function that returns `Pin<Box<dyn Stream<Item = item> + Send + 'lifetime>>`.
If you passed `boxed_local` instead of `boxed`, async stream function
returns a non-thread-safe stream (`Pin<Box<dyn Stream<Item = item> + 'lifetime>>`).

```rust
#![feature(coroutines)]

use std::pin::Pin;

use futures::stream::Stream;
use futures_async_stream::stream;

// The trait itself can be defined without unstable features.
trait Foo {
    fn method(&mut self) -> Pin<Box<dyn Stream<Item = u32> + Send + '_>>;
}

struct Bar(u32);

impl Foo for Bar {
    #[stream(boxed, item = u32)]
    async fn method(&mut self) {
        while self.0 < u32::MAX {
            self.0 += 1;
            yield self.0;
        }
    }
}
```

## `#[try_stream]`

`?` operator can be used with the `#[try_stream]`. The `Item` of the
returned stream is `Result` with `Ok` being the value yielded and `Err` the
error type returned by `?` operator or `return Err(...)`.

```rust
#![feature(coroutines)]

use futures::stream::Stream;
use futures_async_stream::try_stream;

#[try_stream(ok = i32, error = Box<dyn std::error::Error>)]
async fn foo(stream: impl Stream<Item = String>) {
    #[for_await]
    for x in stream {
        yield x.parse()?;
    }
}
```

`#[try_stream]` can be used wherever `#[stream]` can be used.

To early exit from a `#[try_stream]` function or block, use `return Ok(())`.

<!--
## List of features that may be added in the future as an extension of this feature:

- `async_sink` (https://github.com/rust-lang-nursery/futures-rs/pull/1548#issuecomment-486205382)
- Support `.await` in macro (https://github.com/rust-lang-nursery/futures-rs/pull/1548#discussion_r285341883)
- Parallel version of `for_await` (https://github.com/rustasync/runtime/pull/25)
-->

## How to write the equivalent code without this API?

### `#[for_await]`

You can write this by combining `while let` loop, `.await`, `pin!` macro,
and `StreamExt::next()` method:

```rust
use std::pin::pin;

use futures::stream::{Stream, StreamExt};

async fn collect(stream: impl Stream<Item = i32>) -> Vec<i32> {
    let mut vec = vec![];
    let mut stream = pin!(stream);
    while let Some(value) = stream.next().await {
        vec.push(value);
    }
    vec
}
```

### `#[stream]`

You can write this by manually implementing the combinator:

```rust
use std::{
    pin::Pin,
    task::{ready, Context, Poll},
};

use futures::stream::Stream;
use pin_project::pin_project;

fn foo<S>(stream: S) -> impl Stream<Item = i32>
where
    S: Stream<Item = String>,
{
    Foo { stream }
}

#[pin_project]
struct Foo<S> {
    #[pin]
    stream: S,
}

impl<S> Stream for Foo<S>
where
    S: Stream<Item = String>,
{
    type Item = i32;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Some(x) = ready!(self.project().stream.poll_next(cx)) {
            Poll::Ready(Some(x.parse().unwrap()))
        } else {
            Poll::Ready(None)
        }
    }
}
```

[futures-await]: https://github.com/alexcrichton/futures-await

<!-- tidy:crate-doc:end -->

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
