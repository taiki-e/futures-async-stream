#![feature(async_await, generators)]

use futures_async_stream::async_stream;

#[async_stream(item = Option<i32>)]
async fn foo() -> i32 {} //~ ERROR async stream functions must return the unit type

#[async_stream(item = (i32, i32))]
async fn unit() -> () {} // OK

fn main() {}
