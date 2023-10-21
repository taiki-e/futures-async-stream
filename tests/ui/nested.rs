// SPDX-License-Identifier: Apache-2.0 OR MIT

#![feature(coroutines)]

use futures_async_stream::stream;

#[stream(item = i32)]
async fn stream(x: i32) {
    for i in 1..=x {
        yield i
    }
}

#[stream(item = i32)]
async fn _stream1() {
    async {
        #[for_await]
        for i in stream(2) {
            yield i * i; //~ ERROR `async` coroutines are not yet supported [E0727]
        }
    }
    .await;
}

fn main() {}
