// SPDX-License-Identifier: Apache-2.0 OR MIT

#![feature(generators, proc_macro_hygiene, stmt_expr_attributes)]

use futures_async_stream::{for_await, stream};

#[stream(item = i32)]
async fn stream(x: i32) {
    for i in 1..=x {
        yield i
    }
}

async fn async_fn() {
    for _i in 1..2 {
        async {}.await?; //~ ERROR the `?` operator can only be applied to values that implement `std::ops::Try`
    }
}

#[stream(item = i32)]
async fn async_stream_fn() {
    for _i in 1..2 {
        async {}.await?; //~ ERROR the `?` operator can only be applied to values that implement `std::ops::Try`
    }
}

async fn async_fn_and_for_await() {
    #[for_await]
    for _i in stream(2) {
        async {}.await?; //~ ERROR the `?` operator can only be applied to values that implement `std::ops::Try`
    }
}

#[stream(item = i32)]
async fn async_stream_fn_and_for_await() {
    #[for_await]
    for _i in stream(2) {
        async {}.await?; //~ ERROR the `?` operator can only be applied to values that implement `std::ops::Try`
    }
}

fn main() {}
