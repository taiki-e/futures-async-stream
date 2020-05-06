#![warn(rust_2018_idioms, single_use_lifetimes)]
#![allow(incomplete_features)]
#![feature(generators, proc_macro_hygiene, impl_trait_in_bindings)]

use futures::stream::Stream;
use futures_async_stream::{stream, try_stream};

#[stream(item = ())]
async fn stream() {}

#[stream(item = ())]
pub async fn for_await_in_stream_fn() {
    #[for_await]
    for () in stream() {}
}

#[try_stream(ok = (), error = ())]
pub async fn for_await_in_try_stream_fn() {
    #[for_await]
    for () in stream() {}
}

#[stream(item = ())]
pub async fn stream_in_stream_fn() {
    let _: impl Stream<Item = ()> = {
        #[stream]
        async move {}
    };
}

#[try_stream(ok = (), error = ())]
pub async fn stream_in_try_stream_fn() {
    let _: impl Stream<Item = ()> = {
        #[stream]
        async move {}
    };
}

#[stream(item = ())]
pub async fn try_stream_in_stream_fn() {
    let _: impl Stream<Item = Result<(), ()>> = {
        #[try_stream]
        async move {}
    };
}

#[try_stream(ok = (), error = ())]
pub async fn try_stream_in_try_stream_fn() {
    let _: impl Stream<Item = Result<(), ()>> = {
        #[try_stream]
        async move {}
    };
}

#[test]
fn test() {}
