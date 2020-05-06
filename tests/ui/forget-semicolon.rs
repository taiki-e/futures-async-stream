#![feature(generators)]

use futures_async_stream::{stream, try_stream};

#[stream(item = ())]
async fn foo() {
    yield;
    Some(()) //~ ERROR mismatched types
}

#[try_stream(ok = (), error = ())]
async fn bar() {
    yield;
    Some(()) //~ ERROR mismatched types
}

fn main() {}
