#![warn(rust_2018_idioms, single_use_lifetimes)]
#![allow(incomplete_features)] // for impl_trait_in_bindings
#![allow(clippy::try_err)]
#![feature(generators, proc_macro_hygiene, stmt_expr_attributes, impl_trait_in_bindings)]

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
    let s: impl Stream<Item = Result<i32, i32>> = try_stream_block! {
        #[for_await]
        for i in iter(vec![Ok(1), Err(2)]) {
            yield async { i }.await?;
        }
    };
    #[for_await]
    for i in s {
        async { i }.await?;
    }
    Ok::<(), i32>(())
}

pub async fn async_block1() -> Result<(), i32> {
    let s: impl Stream<Item = Result<i32, i32>> = {
        #[try_stream]
        async {
            #[for_await]
            for i in iter(vec![Ok(1), Err(2)]) {
                yield async { i }.await?;
            }
        }
    };
    #[for_await]
    for _i in s {}
    Ok::<(), i32>(())
}

pub async fn async_block2() -> Result<(), i32> {
    let s: impl Stream<Item = Result<i32, i32>> = {
        #[try_stream]
        async move {
            #[for_await]
            for i in iter(vec![Ok(1), Err(2)]) {
                yield async { i }.await?;
            }
        }
    };
    #[for_await]
    for _i in s {}
    Ok::<(), i32>(())
}

#[try_stream(ok = i32, error = i32)]
pub async fn async_block3() {
    let s: impl Stream<Item = Result<i32, i32>> = {
        #[try_stream]
        async move {
            #[for_await]
            for i in iter(vec![Ok(1), Err(2)]) {
                yield async { i }.await?;
            }
        }
    };
    #[for_await]
    for _i in s {}
}

pub async fn async_block_weird_fmt() {
    let _: impl Stream<Item = Result<i32, i32>> = #[try_stream]
    async move {
        #[for_await]
        for i in iter(vec![Ok(1), Err(2)]) {
            yield async { i }.await?;
        }
    };
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
        for x in stream3(v.clone().cloned().take(4)) {
            assert_eq!(x.unwrap(), *v.next().unwrap());
        }

        #[for_await]
        for x in stream3(Vec::new()) {
            assert_eq!(Err(0), x)
        }
    })
}
