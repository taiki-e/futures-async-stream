#![feature(generators)]

use futures_async_stream::async_stream;

#[async_stream(item = Option<i32>)]
async fn foobar() {
    let val = Some(42);
    if val.is_none() {
        yield None;
        return;
    }
    let val = val.unwrap();
    yield val; //~ ERROR mismatched types
}

#[async_stream(item = (i32, i32))]
async fn tuple() {
    if false {
        yield 3;
    }
    yield (1, 2) //~ ERROR mismatched types
}

fn main() {}
