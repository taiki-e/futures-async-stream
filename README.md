# futures-async-stream

[![crates-badge]][crates-url]
[![docs-badge]][docs-url]
[![license-badge]][license]
![rustc-badge]

[crates-badge]: https://img.shields.io/crates/v/futures-async-stream.svg
[crates-url]: https://crates.io/crates/futures-async-stream
[docs-badge]: https://docs.rs/futures-async-stream/badge.svg
[docs-url]: https://docs.rs/futures-async-stream
[license-badge]: https://img.shields.io/crates/l/futures-async-stream.svg
[license]: #license
[rustc-badge]: https://img.shields.io/badge/rustc-nightly-lightgray.svg

Async stream for Rust and the futures crate.

This crate provides useful features for streams, using `async_await` and unstable `generators`.

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
futures-async-stream = "0.1"
futures = "0.3"
```

The current futures-async-stream requires Rust nightly 2019-08-21 or later.

## \#\[for_await\]

Processes streams using a for loop.

This is a reimplement of [futures-await]'s `#[async]` for loops for futures 0.3 and is an experimental implementation of [the idea listed as the next step of async/await](https://github.com/rust-lang/rfcs/blob/master/text/2394-async_await.md#for-await-and-processing-streams).

```rust
#![feature(stmt_expr_attributes, proc_macro_hygiene)]
use futures::stream::Stream;
use futures_async_stream::for_await;

async fn collect(stream: impl Stream<Item = i32>) -> Vec<i32> {
    let mut vec = Vec::new();
    #[for_await]
    for value in stream {
        vec.push(value);
    }
    vec
}
```

`value` has the `Item` type of the stream passed in. Note that async for loops can only be used inside of `async` functions, closures, blocks, `#[async_stream]` functions and `async_stream_block!` macros.

## \#\[async_stream\]

Creates streams via generators.

This is a reimplement of [futures-await]'s `#[async_stream]` for futures 0.3 and is an experimental implementation of [the idea listed as the next step of async/await](https://github.com/rust-lang/rfcs/blob/master/text/2394-async_await.md#generators-and-streams).

```rust
#![feature(generators)]
use futures::stream::Stream;
use futures_async_stream::async_stream;

// Returns a stream of i32
#[async_stream(item = i32)]
async fn foo(stream: impl Stream<Item = String>) {
    // `for_await` is built into `async_stream`. If you use `for_await` only in `async_stream`, there is no need to import `for_await`.
    #[for_await]
    for x in stream {
        yield x.parse().unwrap();
    }
}
```

`#[async_stream]` must have an item type specified via `item = some::Path` and the values output from the stream must be yielded via the `yield` expression.

## async_stream_block!

You can create a stream directly as an expression using an `async_stream_block!` macro:

```rust
#![feature(generators, proc_macro_hygiene)]
use futures::stream::Stream;
use futures_async_stream::async_stream_block;

fn foo() -> impl Stream<Item = i32> {
    async_stream_block! {
        for i in 0..10 {
            yield i;
        }
    }
}
```

## Using async stream functions in traits

You can use async stream functions in traits by passing `boxed` or `boxed_local` as an argument.

```rust
#![feature(generators)]
use futures_async_stream::async_stream;

trait Foo {
    #[async_stream(boxed, item = u32)]
    async fn method(&mut self);
}

struct Bar(u32);

impl Foo for Bar {
    #[async_stream(boxed, item = u32)]
    async fn method(&mut self) {
        while self.0 < u32::max_value() {
            self.0 += 1;
            yield self.0;
        }
    }
}
```

A async stream function that received a `boxed` argument is converted to a function that returns `Pin<Box<dyn Stream<Item = item> + Send + 'lifetime>>`.
If you passed `boxed_local` instead of `boxed`, async stream function returns a non-threadsafe stream (`Pin<Box<dyn Stream<Item = item> + 'lifetime>>`).

```rust
#![feature(generators)]
use futures::stream::Stream;
use futures_async_stream::async_stream;
use std::pin::Pin;

// The trait itself can be defined without unstable features.
trait Foo {
    fn method(&mut self) -> Pin<Box<dyn Stream<Item = u32> + Send + '_>>;
}

struct Bar(u32);

impl Foo for Bar {
    #[async_stream(boxed, item = u32)]
    async fn method(&mut self) {
        while self.0 < u32::max_value() {
            self.0 += 1;
            yield self.0;
        }
    }
}
```

## \#\[async_try_stream\] and async_try_stream_block!

`?` operator can be used with the `#[async_try_stream]` and `async_try_stream_block!`. The `Item` of the returned stream is `Result` with `Ok` being the value yielded and `Err` the error type returned by `?` operator or `return Err(...)`.

```rust
#![feature(generators)]
use futures::stream::Stream;
use futures_async_stream::async_try_stream;

#[async_try_stream(ok = i32, error = Box<dyn std::error::Error + Send + Sync>)]
async fn foo(stream: impl Stream<Item = String>) {
    #[for_await]
    for x in stream {
        yield x.parse()?;
    }
}
```

<!--
## List of features that may be added in the future as an extension of this feature:

  * `async_sink` (https://github.com/rust-lang-nursery/futures-rs/pull/1548#issuecomment-486205382)
  * Support `.await` in macro (https://github.com/rust-lang-nursery/futures-rs/pull/1548#discussion_r285341883)
  * Parallel version of `for_await` (https://github.com/rustasync/runtime/pull/25)
-->

## How to write the equivalent code without this API?

### \#\[for_await\]

You can write this by combining `while let` loop, `.await`, `pin_mut` macro, and `StreamExt::next()` method:

```rust
use futures::{
    pin_mut,
    stream::{Stream, StreamExt},
};

async fn collect(stream: impl Stream<Item = i32>) -> Vec<i32> {
    let mut vec = Vec::new();
    pin_mut!(stream);
    while let Some(value) = stream.next().await {
        vec.push(value);
    }
    vec
}
```

### \#\[async_stream\]

You can write this by manually implementing the combinator:

```rust
use futures::{
    ready,
    stream::Stream,
    task::{Context, Poll},
};
use pin_project::pin_project;
use std::pin::Pin;

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

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
