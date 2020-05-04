#![feature(generators)]
#![allow(unused)]

use futures::{
    executor::block_on,
    future,
    sink::{Sink, SinkExt},
    task::{noop_waker, Context},
};
use futures_async_stream::sink::*;

macro_rules! await_input {
    ($arg:ident) => {{
        let mut item = None;
        loop {
            match item {
                Some(item) => break item,
                None => {
                    match $arg {
                        Arg::StartSend(i) => {
                            item = Some(Some(i));
                            $arg = yield Res::Accepted
                        }
                        Arg::Flush(_cx) => $arg = yield Res::Idle,
                        Arg::Close(cx) => {
                            item = Some(None);
                            $arg = Arg::Close(cx)
                        }
                    };
                }
            }
        }
    }};
}

async fn bar() {
    // future::pending::<()>().await;
    futures::pending!();
    // unreachable!()
}

fn foo() -> impl Sink<String, Error = Complete> {
    from_generator(static move |mut arg: Arg<String>| {
        while let Some(item) = await_input!(arg) {
            dbg!(&item);
            // let _i = bar().await
            let _i = {
                let mut future = futures_async_stream::future::MaybeDone::Future(bar());
                let mut future = unsafe { core::pin::Pin::new_unchecked(&mut future) };
                loop {
                    if let Some(e) = future.as_mut().take_output() {
                        break e;
                    }
                    match arg {
                        Arg::StartSend(item) => arg = yield Res::Pending,
                        Arg::Flush(cx) | Arg::Close(cx) => {
                            let res_ = unsafe {
                                core::future::Future::poll(
                                    future.as_mut(),
                                    futures_async_stream::future::get_context(cx),
                                )
                            };
                            arg = yield Res::Pending
                        }
                    }
                }
            };
        }
    })
}

fn main() {
    let mut sink = Box::pin(foo());
    let w = noop_waker();
    let cx = &mut Context::from_waker(&w);
    dbg!(sink.as_mut().poll_ready(cx));
    dbg!(sink.as_mut().start_send("foo".into()));
    dbg!(sink.as_mut().poll_close(cx));
    dbg!(sink.as_mut().poll_flush(cx));
    dbg!(sink.as_mut().poll_ready(cx));
    dbg!(sink.as_mut().poll_flush(cx));
    dbg!(sink.as_mut().poll_close(cx));
}
