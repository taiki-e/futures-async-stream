#![warn(rust_2018_idioms)]
#![feature(generators, stmt_expr_attributes, proc_macro_hygiene)]

use futures::{
    executor::block_on,
    stream::{self, Stream},
};
use futures_async_stream::{async_stream, async_stream_block, for_await};
use std::{pin::Pin, rc::Rc, sync::Arc};

#[async_stream(item = i32)]
pub async fn nested() {
    let _ = async {
        #[for_await]
        for i in stream::iter(vec![1, 2]) {
            async { i * i }.await;
        }
    };
    async {}.await;
}

pub async fn nested2() {
    let s = async_stream_block! {
        #[for_await]
        for i in stream::iter(vec![1, 2]) {
            yield async { i * i }.await;
        }
    };
    #[for_await]
    for _i in s {
        async {}.await;
    }
}

#[async_stream(item = u64)]
async fn stream1() {
    yield 0;
    yield 1;
}

#[async_stream(item = T)]
pub async fn stream2<T: Clone>(t: T) {
    yield t.clone();
    yield t.clone();
}

#[async_stream(item = i32)]
async fn stream3() {
    let mut cnt = 0;
    #[for_await]
    for x in stream::iter(vec![1, 2, 3, 4]) {
        cnt += x;
        yield x;
    }
    yield cnt;
}

mod foo {
    pub struct _Foo(pub i32);
}

#[async_stream(boxed, item = foo::_Foo)]
pub async fn stream5() {
    yield foo::_Foo(0);
    yield foo::_Foo(1);
}

#[async_stream(item = i32, boxed)]
pub async fn stream6() {
    #[for_await]
    for foo::_Foo(i) in stream5() {
        yield i * i;
    }
}

#[async_stream(boxed_local, item = foo::_Foo)]
pub async fn stream7() {
    yield foo::_Foo(0);
    yield foo::_Foo(1);
}

#[async_stream(item = i32, boxed_local)]
pub async fn stream8() {
    #[for_await]
    for foo::_Foo(i) in stream5() {
        yield i * i;
    }
}

#[async_stream(item = ())]
pub async fn unit() -> () {
    yield ();
}

#[async_stream(item = [u32; 4])]
pub async fn array() {
    yield [1, 2, 3, 4];
}

pub struct A(i32);

impl A {
    #[async_stream(item = i32)]
    pub async fn take_self(self) {
        yield self.0
    }

    #[async_stream(item = i32)]
    pub async fn take_ref_self(&self) {
        yield self.0
    }

    #[async_stream(item = i32)]
    pub async fn take_ref_mut_self(&mut self) {
        yield self.0
    }

    #[async_stream(item = i32)]
    pub async fn take_box_self(self: Box<Self>) {
        yield self.0
    }

    #[async_stream(item = i32)]
    pub async fn take_rc_self(self: Rc<Self>) {
        yield self.0
    }

    #[async_stream(item = i32)]
    pub async fn take_arc_self(self: Arc<Self>) {
        yield self.0
    }

    #[async_stream(item = i32)]
    pub async fn take_pin_ref_self(self: Pin<&Self>) {
        yield self.0
    }

    #[async_stream(item = i32)]
    pub async fn take_pin_ref_mut_self(self: Pin<&mut Self>) {
        yield self.0
    }

    #[async_stream(item = i32)]
    pub async fn take_pin_box_self(self: Pin<Box<Self>>) {
        yield self.0
    }
}

pub trait Trait {
    fn stream1() -> Pin<Box<dyn Stream<Item = i32> + Send>>;

    fn stream2(&self) -> Pin<Box<dyn Stream<Item = i32> + Send + '_>>;

    #[async_stream(boxed, item = i32)]
    async fn stream3();

    #[async_stream(boxed, item = i32)]
    async fn stream4(&self);
}

impl Trait for A {
    #[async_stream(boxed, item = i32)]
    async fn stream1() {
        yield 1;
    }

    #[async_stream(boxed, item = i32)]
    async fn stream2(&self) {
        yield 1;
    }

    #[async_stream(boxed, item = i32)]
    async fn stream3() {
        yield 1;
    }

    #[async_stream(boxed, item = i32)]
    async fn stream4(&self) {
        yield 1;
    }
}

#[test]
fn test() {
    // https://github.com/alexcrichton/futures-await/issues/45
    #[async_stream(item = ())]
    async fn _stream10() {
        yield;
    }

    block_on(async {
        let mut v = 0..=1;
        #[for_await]
        for x in stream1() {
            assert_eq!(x, v.next().unwrap());
        }

        let mut v = [1, 2, 3, 4, 10].iter();
        #[for_await]
        for x in stream3() {
            assert_eq!(x, *v.next().unwrap());
        }

        #[for_await]
        for x in A(11).take_self() {
            assert_eq!(x, 11);
        }
    });
}
