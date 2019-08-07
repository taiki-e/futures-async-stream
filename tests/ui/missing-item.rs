// compile-fail

#![deny(warnings)]
#![feature(async_await, generators)]

use futures_async_stream::async_stream;

#[async_stream] //~ ERROR unexpected end of input, expected `item`
async fn foo(a: String) {}

fn main() {}
