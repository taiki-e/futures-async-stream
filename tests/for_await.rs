#![warn(rust_2018_idioms, single_use_lifetimes)]
#![feature(generators, proc_macro_hygiene, stmt_expr_attributes)]

use futures::{
    future::Future,
    pin_mut,
    stream::{self, Stream},
    task::{noop_waker, Context, Poll},
};
use futures_async_stream::{for_await, stream, stream_block};

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

async fn in_async_fn() -> i32 {
    let mut cnt = 0;
    #[for_await]
    for x in stream::iter(vec![1, 2, 3, 4]) {
        cnt += x;
    }
    cnt
}

async fn nested() -> bool {
    let mut cnt = 0;
    let vec = vec![1, 2, 3, 4];
    #[for_await]
    for x in iter(vec.clone()) {
        #[for_await]
        for y in iter(vec.clone()) {
            cnt += x * y;
        }
    }

    let sum = (1..5).map(|x| (1..5).map(|y| x * y).sum::<i32>()).sum::<i32>();
    cnt == sum
}

#[stream(item = i32)]
pub async fn in_stream_fn() {
    #[for_await]
    for i in iter(1..10) {
        yield i * i;
    }
}

pub fn in_stream_block() -> impl Stream<Item = i32> {
    stream_block! {
        #[for_await]
        for item in in_stream_fn() {
            yield item
        }
    }
}

#[test]
fn test() {
    run(async {
        assert_eq!(in_async_fn().await, 10);
        assert_eq!(nested().await, true);
    })
}

#[allow(single_use_lifetimes)]
pub fn pat_ref<'life0, 'life1, 'life2, 'async_stream, T>(
    _x: &'life0 T,
    _y: (&'life1 i32, &'life2 i8),
) -> impl ::futures_async_stream::__reexport::stream::Stream<Item = ()>
+ 'life0 + 'life1 + 'life2 + 'async_stream
where
    T: 'async_stream,
    'life0: 'async_stream,
    'life1: 'async_stream,
    'life2: 'async_stream,
    &'life0 T: 'async_stream,
    (&'life1 i32, &'life2 i8): 'async_stream,
{
    :: futures_async_stream :: __reexport :: stream :: from_generator (
        static move | mut __task_context : :: futures_async_stream :: __reexport :: future :: ResumeTy | -> ( ) {
            let ( ) : ( ) = {
                // let mut _y = _y;
                     let _y = _y;
                __task_context = ( yield :: futures_async_stream :: __reexport :: task :: Poll :: Ready ( ( ) ) ) } ;
                # [ allow ( unreachable_code ) ] { return ; loop { __task_context = ( yield :: futures_async_stream :: __reexport :: task :: Poll :: Pending ) } } } )
}

// #[allow(single_use_lifetimes)]
// pub fn check_for_name_collision<'_async0, 'life0, 'async_stream, T>(
//     _x: &'life0 T,
//     _y: &'_async0 i32,
// ) -> impl ::futures_async_stream::__reexport::stream::Stream<Item = ()> + '_async0 + 'life0 + 'async_stream
// where
//     '_async0: 'async_stream,
//     T: 'async_stream,
//     'life0: 'async_stream,
// {
//     :: futures_async_stream :: __reexport :: stream :: from_generator (
//          static move | mut __task_context : :: futures_async_stream :: __reexport :: future :: ResumeTy | -> ( ) {

//               let ( ) : ( ) = {
//                   __task_context = ( yield :: futures_async_stream :: __reexport :: task :: Poll :: Ready ( ( ) ) ) } ;
//                    # [ allow ( unreachable_code ) ] { return ; loop { __task_context = ( yield :: futures_async_stream :: __reexport :: task :: Poll :: Pending ) } }
//         } )
// }
