#![warn(rust_2018_idioms)]
#![feature(generators, stmt_expr_attributes, proc_macro_hygiene)]

use futures::{
    executor::block_on,
    stream::{self, Stream},
};
use futures_async_stream::{async_stream, async_stream_block, for_await};

async fn in_async_fn() -> i32 {
    let mut cnt = 0;
    #[for_await]
    for x in stream::iter(vec![1, 2, 3, 4]) {
        cnt += x;
    }
    cnt
}

async fn nested() -> bool {
    let mut cnt = 0;
    let vec = vec![1, 2, 3, 4];
    #[for_await]
    for x in stream::iter(vec.clone()) {
        #[for_await]
        for y in stream::iter(vec.clone()) {
            cnt += x * y;
        }
    }

    let sum = (1..5).map(|x| (1..5).map(|y| x * y).sum::<i32>()).sum::<i32>();
    cnt == sum
}

#[async_stream(item = i32)]
pub async fn in_async_stream_fn() {
    #[for_await]
    for i in stream::iter(1..10) {
        yield i * i;
    }
}

pub fn in_async_stream_block() -> impl Stream<Item = i32> {
    async_stream_block! {
        #[for_await]
        for item in in_async_stream_fn() {
            yield item
        }
    }
}

#[test]
fn test() {
    assert_eq!(block_on(in_async_fn()), 10);
    assert_eq!(block_on(nested()), true);
}
