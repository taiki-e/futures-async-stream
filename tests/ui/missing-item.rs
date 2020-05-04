#![feature(generators)]

use futures_async_stream::stream;

#[stream] //~ ERROR unexpected end of input, expected `item`
async fn foo(a: String) {}

fn main() {}
