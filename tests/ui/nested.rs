// compile-fail

#![feature(generators)]

use futures_async_stream::async_stream;

#[async_stream(item = i32)]
async fn stream(x: i32) {
    for i in 1..=x {
        yield i
    }
}

#[async_stream(item = i32)]
async fn _stream1() {
    async {
        #[for_await]
        for i in stream(2) {
            yield i * i; //~ ERROR `async` generators are not yet supported [E0727]
        }
    }
        .await;
}

fn main() {}
