// compile-fail

#![deny(warnings)]
#![feature(generators)]

use futures_async_stream::async_stream;

#[async_stream(item = ())]
async fn foo() {
    yield;
    Some(()) //~ ERROR mismatched types
}

fn main() {}
