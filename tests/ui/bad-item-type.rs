// SPDX-License-Identifier: Apache-2.0 OR MIT

#![feature(generators)]

use futures_async_stream::stream;

#[stream(item = Option<i32>)]
async fn option() {
    let val = Some(42);
    if val.is_none() {
        yield None;
        return;
    }
    let val = val.unwrap();
    yield val; //~ ERROR mismatched types
}

#[stream(item = Option<i32>, boxed)]
async fn option_boxed() {
    let val = Some(42);
    if val.is_none() {
        yield None;
        return;
    }
    let val = val.unwrap();
    yield val; //~ ERROR mismatched types
}

#[stream(item = (i32, i32))]
async fn tuple() {
    if false {
        yield 3;
    }
    yield (1, 2) //~ ERROR mismatched types
}

#[stream(item = (i32, i32), boxed)]
async fn tuple_boxed() {
    if false {
        yield 3;
    }
    yield (1, 2) //~ ERROR mismatched types
}

fn main() {}
