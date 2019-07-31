#![feature(async_await, generators)]

use futures_async_stream::async_stream;

#[async_stream(item = ())]
fn _stream1() {} //~ ERROR async_stream can only be applied to async functions

fn main() {}
