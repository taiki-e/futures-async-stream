// SPDX-License-Identifier: Apache-2.0 OR MIT

#![feature(coroutines, proc_macro_hygiene, stmt_expr_attributes)]

use std::pin::pin;

use futures::{
    future::Future,
    stream::{self, Stream},
    task::{Context, Poll, noop_waker},
};
use futures_async_stream::{for_await, stream, stream_block};

fn run<F: Future>(f: F) -> F::Output {
    let w = noop_waker();
    let cx = &mut Context::from_waker(&w);
    let mut f = pin!(f);
    loop {
        if let Poll::Ready(x) = f.as_mut().poll(cx) {
            return x;
        }
    }
}

#[stream(item = T)]
async fn iter<T>(iter: impl IntoIterator<Item = T>) {
    for x in iter {
        yield x;
    }
}

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
    for x in iter(vec.clone()) {
        #[for_await]
        for y in iter(vec.clone()) {
            cnt += x * y;
        }
    }

    let sum = (1..5).map(|x| (1..5).map(|y| x * y).sum::<i32>()).sum::<i32>();
    cnt == sum
}

#[stream(item = i32)]
pub async fn in_stream_fn() {
    #[for_await]
    for i in iter(1..10) {
        yield i * i;
    }
}

pub fn in_stream_block() -> impl Stream<Item = i32> {
    stream_block! {
        #[for_await]
        for item in in_stream_fn() {
            yield item;
        }
    }
}

#[test]
fn test() {
    run(async {
        assert_eq!(in_async_fn().await, 10);
        assert!(nested().await);
    });
}
