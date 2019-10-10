#![feature(generators)]

use futures_async_stream::async_stream;

#[async_stream(item = Left)] //~ ERROR cannot find type `Left` in this scope
async fn foo() {}

fn main() {}
