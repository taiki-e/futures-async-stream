// compile-fail

#![deny(warnings)]
#![feature(generators)]

use futures::stream;
use futures_async_stream::async_stream;

#[async_stream(item = i32)]
async fn a() {
    #[for_await(bar)] //~ ERROR unexpected token
    for i in stream::iter(vec![1, 2]) {
        yield i;
    }
}

#[async_stream(baz, item = i32)] //~ ERROR expected `item`
async fn b() {
    yield 1;
}

#[async_stream(item = i32, baz)] //~ ERROR unexpected token
async fn c() {
    yield 1;
}

#[async_stream(item = i32 item = i32)] //~ ERROR expected `,`
async fn d() {
    yield 1;
}

#[async_stream(item = i32, item = i32)] //~ ERROR duplicate `item` argument
async fn duplicate_item() {
    yield 1;
}

#[async_stream(item = i32, boxed, boxed)] //~ ERROR duplicate `boxed` argument
async fn duplicate_boxed() {
    yield 1;
}

#[async_stream(item = i32, boxed_local, boxed_local)] //~ ERROR duplicate `boxed_local` argument
async fn duplicate_boxed_local() {
    yield 1;
}

#[async_stream(item = i32, boxed_local, boxed)] //~ ERROR `boxed` and `boxed_local` cannot be used at the same time.
async fn combine() {
    yield 1;
}

fn main() {}
