#![warn(rust_2018_idioms)]
#![feature(generators, stmt_expr_attributes, proc_macro_hygiene, impl_trait_in_bindings)]
#![allow(incomplete_features)]

use futures::{
    executor::block_on,
    stream::{self, Stream},
};
use futures_async_stream::{async_try_stream, async_try_stream_block, for_await};

#[async_try_stream(ok = i32, error = i32)]
pub async fn nested() {
    let f = async {
        #[for_await]
        for i in stream::iter(vec![Ok(1), Err(2)]) {
            async { i }.await?;
        }
        Ok::<(), i32>(())
    };
    f.await?;
}

pub async fn nested2() -> Result<(), i32> {
    let s: impl Stream<Item = Result<i32, i32>> = async_try_stream_block! {
        #[for_await]
        for i in stream::iter(vec![Ok(1), Err(2)]) {
            yield async { i }.await?;
        }
    };
    #[for_await]
    for i in s {
        async { i }.await?;
    }
    Ok::<(), i32>(())
}

#[async_try_stream(ok = u64, error = i32)]
async fn stream1() {
    yield 0;
    yield Err(1)?;
}

#[async_try_stream(ok = u32, error = i32)]
async fn stream3(iter: impl IntoIterator<Item = u32>) {
    let mut sum = 0;
    #[for_await]
    for x in stream::iter(iter) {
        sum += x;
        yield x;
    }
    if sum == 0 {
        return Err(0);
    }
    yield sum;
}

#[async_try_stream(ok = i32, error = Box<dyn std::error::Error + Send + Sync + 'static>)]
pub async fn dyn_trait(stream: impl Stream<Item = String>) {
    #[for_await]
    for x in stream {
        yield x.parse()?;
    }
}

#[test]
fn test() {
    async fn foo() {
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
    }
    let _ = block_on(foo());
}
