// SPDX-License-Identifier: Apache-2.0 OR MIT

#![feature(coroutines)]

use futures_async_stream::{stream, try_stream};

#[stream(item = ())]
async fn foo() {
    yield;
    Some(()) //~ ERROR mismatched types
}

#[try_stream(ok = (), error = ())]
async fn bar() {
    yield;
    Some(()) //~ ERROR mismatched types
}

fn main() {}
