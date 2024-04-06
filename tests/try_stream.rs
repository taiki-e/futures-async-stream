// SPDX-License-Identifier: Apache-2.0 OR MIT

#![allow(unreachable_pub, clippy::try_err, clippy::unused_async)]
#![feature(coroutines, proc_macro_hygiene, stmt_expr_attributes)]

use futures::{
    future::Future,
    pin_mut,
    stream::Stream,
    task::{noop_waker, Context, Poll},
};
use futures_async_stream::{for_await, stream, try_stream, try_stream_block};

fn run<F: Future>(f: F) -> F::Output {
    let w = noop_waker();
    let cx = &mut Context::from_waker(&w);
    pin_mut!(f);
    loop {
        if let Poll::Ready(x) = f.as_mut().poll(cx) {
            return x;
        }
    }
}

// TODO: allow specifying error type in #[try_stream] attribute and remove this hack.
fn ensure_item_type<T, E, S: Stream<Item = Result<T, E>>>(s: S) -> S {
    s
}

#[stream(item = T)]
async fn iter<T>(iter: impl IntoIterator<Item = T>) {
    for x in iter {
        yield x;
    }
}

#[try_stream(ok = i32, error = i32)]
pub async fn nested() {
    let f = async {
        #[for_await]
        for i in iter(vec![Ok(1), Err(2)]) {
            async { i }.await?;
        }
        Ok::<(), i32>(())
    };
    f.await?;
}

pub async fn nested2() -> Result<(), i32> {
    let s = try_stream_block! {
        #[for_await]
        for i in iter(vec![Ok(1), Err(2)]) {
            yield async { i }.await?;
        }
    };
    let s = ensure_item_type::<i32, i32, _>(s);
    #[for_await]
    for i in s {
        async { i }.await?;
    }
    Ok::<(), i32>(())
}

pub async fn async_block1() -> Result<(), i32> {
    let s = {
        #[try_stream]
        async {
            #[for_await]
            for i in iter(vec![Ok(1), Err(2)]) {
                yield async { i }.await?;
            }
        }
    };
    let s = ensure_item_type::<i32, i32, _>(s);
    #[for_await]
    for _i in s {}
    Ok::<(), i32>(())
}

pub async fn async_block2() -> Result<(), i32> {
    let s = {
        #[try_stream]
        async move {
            #[for_await]
            for i in iter(vec![Ok(1), Err(2)]) {
                yield async { i }.await?;
            }
        }
    };
    let s = ensure_item_type::<i32, i32, _>(s);
    #[for_await]
    for _i in s {}
    Ok::<(), i32>(())
}

#[try_stream(ok = i32, error = i32)]
pub async fn async_block3() {
    let s = {
        #[try_stream]
        async move {
            #[for_await]
            for i in iter(vec![Ok(1), Err(2)]) {
                yield async { i }.await?;
            }
        }
    };
    let s = ensure_item_type::<i32, i32, _>(s);
    #[for_await]
    for _i in s {}
}

pub async fn async_block_weird_fmt() {
    let s = #[try_stream]
    async move {
        #[for_await]
        for i in iter(vec![Ok(1), Err(2)]) {
            yield async { i }.await?;
        }
    };
    let _ = ensure_item_type::<i32, i32, _>(s);
}

#[try_stream(ok = u64, error = i32)]
async fn stream1() {
    yield 0;
    yield Err(1)?;
}

#[try_stream(ok = u32, error = i32)]
async fn stream3(iter1: impl IntoIterator<Item = u32>) {
    let mut sum = 0;
    #[for_await]
    for x in iter(iter1) {
        sum += x;
        yield x;
    }
    if sum == 0 {
        return Err(0);
    }
    yield sum;
}

#[try_stream(ok = i32, error = Box<dyn std::error::Error + Send + Sync + 'static>)]
pub async fn dyn_trait(stream: impl Stream<Item = String>) {
    #[for_await]
    for x in stream {
        yield x.parse()?;
    }
}

#[test]
fn test() {
    run(async {
        let mut v = [Ok(0), Err(1)].iter();
        #[for_await]
        for x in stream1() {
            assert_eq!(x, *v.next().unwrap());
        }

        let mut v = [1, 2, 3, 4, 10].iter();
        #[for_await]
        for x in stream3(v.clone().copied().take(4)) {
            assert_eq!(x.unwrap(), *v.next().unwrap());
        }

        #[for_await]
        for x in stream3(vec![]) {
            assert_eq!(Err(0), x);
        }
    });
}

#[test]
fn test_early_exit() {
    #[try_stream(ok = i32, error = i32)]
    async fn early_exit() {
        for i in 0..10 {
            if i == 5 {
                return Ok(());
            }
            yield i;
        }
    }

    #[try_stream(ok = i32, error = i32)]
    pub async fn early_exit_block() {
        let s = try_stream_block! {
            for i in 0..10 {
                if i == 5 {
                    // This will exit the block, not the function.
                    return Ok(());
                }
                yield i;
            }
        };
        let s = ensure_item_type::<i32, i32, _>(s);

        #[for_await]
        for i in s {
            yield i? + 1
        }
    }

    run(async {
        let mut v = 0..5;
        #[for_await]
        for x in early_exit() {
            assert_eq!(x.unwrap(), v.next().unwrap());
        }

        let mut v = 1..6;
        #[for_await]
        for x in early_exit_block() {
            assert_eq!(x.unwrap(), v.next().unwrap());
        }
    });
}
