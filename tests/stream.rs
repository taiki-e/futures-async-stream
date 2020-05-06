#![warn(rust_2018_idioms, single_use_lifetimes)]
#![feature(generators, proc_macro_hygiene, stmt_expr_attributes)]

use futures::{
    future::Future,
    pin_mut,
    stream::Stream,
    task::{noop_waker, Context, Poll},
};
use futures_async_stream::{for_await, stream, stream_block};
use std::{pin::Pin, rc::Rc, sync::Arc};

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

#[stream(item = i32)]
async fn stream(x: i32) {
    for i in 1..=x {
        yield i
    }
}

#[stream(item = i32)]
pub async fn nested() {
    let _ = async {
        #[for_await]
        for i in stream(2) {
            async { i * i }.await;
        }
    };
    async {}.await;
}

pub async fn nested2() {
    let s = stream_block! {
        #[for_await]
        for i in stream(2) {
            yield async { i * i }.await;
        }
    };
    #[for_await]
    for _i in s {
        async {}.await;
    }
}

pub async fn async_block1() {
    let s = {
        #[stream]
        async {
            #[for_await]
            for i in stream(2) {
                yield async { i * i }.await;
            }
        }
    };
    #[for_await]
    for _i in s {}
}

pub async fn async_block2() {
    let s = {
        #[stream]
        async move {
            #[for_await]
            for i in stream(2) {
                yield async { i * i }.await;
            }
        }
    };
    #[for_await]
    for _i in s {}
}

#[stream(item = u64)]
pub async fn async_block3() {
    let s = {
        #[stream]
        async move {
            #[for_await]
            for i in stream(2) {
                yield async { i * i }.await;
            }
        }
    };
    #[for_await]
    for _i in s {}
}

pub async fn async_block_weird_fmt() {
    let _s = #[stream]
    async move {
        #[for_await]
        for i in stream(2) {
            yield async { i * i }.await;
        }
    };
}

#[stream(item = u64)]
async fn stream1() {
    yield 0;
    yield 1;
}

#[stream(item = T)]
pub async fn stream2<T: Clone>(t: T) {
    yield t.clone();
    yield t.clone();
}

#[stream(item = i32)]
async fn stream3() {
    let mut cnt = 0;
    #[for_await]
    for x in stream(4) {
        cnt += x;
        yield x;
    }
    yield cnt;
}

mod foo {
    pub struct _Foo(pub i32);
}

#[stream(boxed, item = foo::_Foo)]
pub async fn stream5() {
    yield foo::_Foo(0);
    yield foo::_Foo(1);
}

#[stream(item = i32, boxed)]
pub async fn stream6() {
    #[for_await]
    for foo::_Foo(i) in stream5() {
        yield i * i;
    }
}

#[stream(boxed_local, item = foo::_Foo)]
pub async fn stream7() {
    yield foo::_Foo(0);
    yield foo::_Foo(1);
}

#[stream(item = i32, boxed_local)]
pub async fn stream8() {
    #[for_await]
    for foo::_Foo(i) in stream5() {
        yield i * i;
    }
}

#[stream(item = ())]
pub async fn unit() -> () {
    yield ();
}

#[stream(item = [u32; 4])]
pub async fn array() {
    yield [1, 2, 3, 4];
}

#[allow(clippy::toplevel_ref_arg)]
pub mod arguments {
    use super::*;

    #[stream(item = ())]
    pub async fn arg_ref(ref x: u8) {
        let _x = x;
    }

    #[stream(item = ())]
    pub async fn arg_mut(mut x: u8) {
        let _x = &mut x;
    }

    #[stream(item = ())]
    pub async fn arg_ref_mut(ref mut x: u8) {
        let _x = x;
    }
}

pub struct Receiver(i32);

impl Receiver {
    #[stream(item = i32)]
    pub async fn take_self(self) {
        yield self.0
    }

    #[stream(item = i32)]
    pub async fn take_ref_self(&self) {
        yield self.0
    }

    #[stream(item = i32)]
    pub async fn take_ref_mut_self(&mut self) {
        yield self.0
    }

    #[stream(item = i32)]
    pub async fn take_box_self(self: Box<Self>) {
        yield self.0
    }

    #[stream(item = i32)]
    pub async fn take_rc_self(self: Rc<Self>) {
        yield self.0
    }

    #[stream(item = i32)]
    pub async fn take_arc_self(self: Arc<Self>) {
        yield self.0
    }

    #[stream(item = i32)]
    pub async fn take_pin_ref_self(self: Pin<&Self>) {
        yield self.0
    }

    #[stream(item = i32)]
    pub async fn take_pin_ref_mut_self(self: Pin<&mut Self>) {
        yield self.0
    }

    #[stream(item = i32)]
    pub async fn take_pin_box_self(self: Pin<Box<Self>>) {
        yield self.0
    }
}

pub trait Trait {
    fn stream1() -> Pin<Box<dyn Stream<Item = i32> + Send>>;

    fn stream2(&self) -> Pin<Box<dyn Stream<Item = i32> + Send + '_>>;

    #[stream(boxed, item = i32)]
    async fn stream3();

    #[stream(boxed, item = i32)]
    async fn stream4(&self);
}

pub struct A(i32);

impl Trait for A {
    #[stream(boxed, item = i32)]
    async fn stream1() {
        yield 1;
    }

    #[stream(boxed, item = i32)]
    async fn stream2(&self) {
        yield 1;
    }

    #[stream(boxed, item = i32)]
    async fn stream3() {
        yield 1;
    }

    #[stream(boxed, item = i32)]
    async fn stream4(&self) {
        yield 1;
    }
}

#[test]
fn test() {
    // https://github.com/alexcrichton/futures-await/issues/45
    #[stream(item = ())]
    async fn _stream10() {
        yield;
    }

    run(async {
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
        for x in Receiver(11).take_self() {
            assert_eq!(x, 11);
        }
    })
}
