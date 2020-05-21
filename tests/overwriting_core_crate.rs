#![warn(rust_2018_idioms, single_use_lifetimes)]
#![allow(incomplete_features)]
#![feature(generators, impl_trait_in_bindings)]

// See https://github.com/rust-lang/pin-utils/pull/26#discussion_r344491597
//
// Note: If the proc-macro does not depend on its own items, it may be preferable not to
//       support overwriting the name of core/std crate for compatibility with reexport.
#[allow(unused_extern_crates)]
extern crate futures_async_stream as core;

// Dummy module to check that the expansion refers to the crate.
mod futures_async_stream {}

use ::futures_async_stream::{stream, try_stream};
use futures_core::stream::Stream;

#[stream(item = ())]
async fn stream() {}

#[stream(item = ())]
pub async fn for_await_in_stream_fn() {
    #[for_await]
    for () in stream() {
        yield;
        async {}.await;
    }
    yield;
    async {}.await;
}

#[try_stream(ok = (), error = ())]
pub async fn for_await_in_try_stream_fn() {
    #[for_await]
    for () in stream() {
        yield;
        async {}.await;
    }
    yield;
    async {}.await;
}

#[stream(item = ())]
pub async fn stream_in_stream_fn() {
    let _: impl Stream<Item = ()> = {
        #[stream]
        async move {
            yield;
            async {}.await;
        }
    };
    yield;
    async {}.await;
}

#[try_stream(ok = (), error = ())]
pub async fn stream_in_try_stream_fn() {
    let _: impl Stream<Item = ()> = {
        #[stream]
        async move {
            yield;
            async {}.await;
        }
    };
    yield;
    async {}.await;
}

#[stream(item = ())]
pub async fn try_stream_in_stream_fn() {
    let _: impl Stream<Item = Result<(), ()>> = {
        #[try_stream]
        async move {
            yield;
            async {}.await;
        }
    };
    yield;
    async {}.await;
}

#[try_stream(ok = (), error = ())]
pub async fn try_stream_in_try_stream_fn() {
    let _: impl Stream<Item = Result<(), ()>> = {
        #[try_stream]
        async move {
            yield;
            async {}.await;
        }
    };
    yield;
    async {}.await;
}

#[test]
fn test() {}
