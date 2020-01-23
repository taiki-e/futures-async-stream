#![warn(rust_2018_idioms)]
#![allow(clippy::trivially_copy_pass_by_ref)]
#![feature(generators)]

use futures_async_stream::async_stream;

pub struct Ref<'a, T>(&'a T);

#[async_stream(item = i32)]
pub async fn references(x: &i32) {
    yield *x;
}

#[async_stream(item = i32)]
pub async fn new_types(x: Ref<'_, i32>) {
    yield *x.0;
}

pub struct Foo(i32);

impl Foo {
    #[async_stream(item = &i32)]
    pub async fn foo(&self) {
        yield &self.0
    }
}

#[async_stream(item = &i32)]
pub async fn single_ref(x: &i32) {
    yield x
}

#[async_stream(item = ())]
pub async fn check_for_name_colision<'_async0, T>(_x: &T, _y: &'_async0 i32) {
    yield
}

#[test]
fn test() {}
