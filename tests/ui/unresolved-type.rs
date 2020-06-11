#![feature(generators)]

use futures_async_stream::stream;

#[stream(item = L)] //~ ERROR cannot find type `Left` in this scope
async fn foo() {}

fn main() {}
