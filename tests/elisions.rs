// SPDX-License-Identifier: Apache-2.0 OR MIT

#![warn(rust_2018_idioms, single_use_lifetimes)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::needless_lifetimes)] // broken
#![feature(coroutines)]

use futures_async_stream::stream;

pub struct Ref<'a, T>(&'a T);

#[stream(item = i32)]
pub async fn references(x: &i32) {
    yield *x;
}

#[stream(item = i32)]
pub async fn new_types(x: Ref<'_, i32>) {
    yield *x.0;
}

pub struct Foo(i32);

impl Foo {
    #[stream(item = &i32)]
    pub async fn foo(&self) {
        yield &self.0;
    }
}

#[stream(item = &i32)]
pub async fn single_ref(x: &i32) {
    yield x;
}

#[stream(item = ())]
pub async fn multi_ref<T>(_x: &T, _y: &i32) {
    yield;
}

#[stream(item = ())]
pub async fn pat_ref<T>(_x: &T, _y: (&i32, &i8)) {
    yield;
}

#[allow(single_use_lifetimes)]
#[stream(item = ())]
pub async fn check_for_name_collision<'_async0, T>(_x: &T, _y: &'_async0 i32) {
    yield;
}

pub trait A<'a> {}

#[allow(single_use_lifetimes)]
#[stream(item = ())]
pub async fn check_for_name_collision2<T>(_x: &T)
where
    for<'_async0> &'_async0 T: Unpin,
    T: for<'_async0> A<'_async0>,
{
    yield;
}
