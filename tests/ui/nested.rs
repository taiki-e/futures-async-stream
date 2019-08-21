// compile-fail

#![deny(warnings)]
#![feature(generators)]

use futures::stream;
use futures_async_stream::async_stream;

#[async_stream(item = i32)]
async fn _stream1() {
    async {
        #[for_await]
        for i in stream::iter(vec![1, 2]) {
            yield i * i; //~ ERROR `async` generators are not yet supported [E0727]
        }
    }
        .await;
}

fn main() {}
