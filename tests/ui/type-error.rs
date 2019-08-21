// compile-fail

#![deny(warnings)]
#![feature(generators)]

use futures_async_stream::async_stream;

#[async_stream(item = i32)]
async fn foo() {
    let a: i32 = "a"; //~ ERROR mismatched types
    yield 1;
}

fn main() {}
