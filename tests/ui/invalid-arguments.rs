#![feature(async_await, generators)]

use futures::stream;
use futures_async_stream::async_stream;

#[async_stream(item = i32)]
async fn foo() {
    #[for_await(bar)] //~ ERROR unexpected token
    for i in stream::iter(vec![1, 2]) {
        yield i;
    }
}

#[async_stream(baz, item = i32)] //~ ERROR expected `item`
async fn bar() {
    #[for_await]
    for i in stream::iter(vec![1, 2]) {
        yield i;
    }
}

#[async_stream(item = i32, baz)] //~ ERROR unexpected token
async fn baz() {
    #[for_await]
    for i in stream::iter(vec![1, 2]) {
        yield i;
    }
}

fn main() {}
