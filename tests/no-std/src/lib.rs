#![no_std]
#![warn(rust_2018_idioms, single_use_lifetimes)]
#![feature(generators)]

use futures_async_stream::stream;

#[stream(item = T)]
async fn iter<T>(iter: impl IntoIterator<Item = T>) {
    for x in iter {
        yield x;
    }
}

#[stream(item = i32)]
pub async fn stream() {
    let mut cnt = 0;
    #[for_await]
    for x in iter(1..4) {
        cnt += x;
        yield x;
    }
    yield cnt;
}
