#![feature(generators)]

use futures_async_stream::stream;

#[stream(item = i32)]
async fn foo() {
    let a: i32 = "a"; //~ ERROR mismatched types
    yield 1;
}

fn main() {}
