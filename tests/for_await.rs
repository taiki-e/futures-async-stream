#![warn(rust_2018_idioms)]
#![feature(generators, stmt_expr_attributes, proc_macro_hygiene)]

use futures::{executor::block_on, stream::Stream};
use futures_async_stream::{async_stream, async_stream_block, for_await};

#[async_stream(item = T)]
async fn iter<T>(iter: impl IntoIterator<Item = T>) {
    for x in iter {
        yield x;
    }
}

async fn in_async_fn() -> i32 {
    let mut cnt = 0;
    #[for_await]
    for x in iter(vec![1, 2, 3, 4]) {
        cnt += x;
    }
    cnt
}

async fn nested() -> bool {
    let mut cnt = 0;
    let vec = vec![1, 2, 3, 4];
    #[for_await]
    for x in iter(vec.clone()) {
        #[for_await]
        for y in iter(vec.clone()) {
            cnt += x * y;
        }
    }

    let sum = (1..5).map(|x| (1..5).map(|y| x * y).sum::<i32>()).sum::<i32>();
    cnt == sum
}

#[async_stream(item = i32)]
pub async fn in_async_stream_fn() {
    #[for_await]
    for i in iter(1..10) {
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

pub fn for_await_syntax() -> impl Stream<Item = i32> {
    async_stream_block! {
        for await item in in_async_stream_fn() {
            yield item
        }
    }
}

#[test]
fn test() {
    block_on(async {
        assert_eq!(in_async_fn().await, 10);
        assert_eq!(nested().await, true);
    })
}
