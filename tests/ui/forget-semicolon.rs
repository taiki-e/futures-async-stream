#![feature(generators)]

use futures_async_stream::stream;

#[stream(item = ())]
async fn foo() {
    yield;
    Some(()) //~ ERROR mismatched types
}

fn main() {}
