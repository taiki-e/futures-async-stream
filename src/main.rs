#![feature(generators)]
#![allow(unused)]

use futures::{
    executor::block_on,
    future::{self, Future},
    pin_mut,
    sink::{Sink, SinkExt},
    stream::{self, Stream},
    task::{noop_waker, Context, Poll},
};
use futures_async_stream::{sink::*, stream};
use std::convert::Infallible;

fn run<F: Future + Unpin>(f: F) -> F::Output {
    let w = noop_waker();
    let cx = &mut Context::from_waker(&w);
    let mut f = Box::pin(f);
    loop {
        if let Poll::Ready(x) = f.as_mut().poll(cx) {
            return x;
        }
    }
}

macro_rules! await_input {
    ($arg:ident) => {{
        let mut item = None;
        loop {
            dbg!(&item);
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

macro_rules! await_in_sink {
    ($arg:ident, $expr:expr) => {{
        let mut future = futures_async_stream::__reexport::future::MaybeDone::Future($expr);
        let mut future = unsafe { core::pin::Pin::new_unchecked(&mut future) };
        loop {
            if let Some(e) = future.as_mut().take_output() {
                break e;
            }
            match $arg {
                Arg::StartSend(item) => $arg = yield Res::Pending,
                Arg::Flush(cx) | Arg::Close(cx) => {
                    let res = unsafe {
                        core::future::Future::poll(
                            future.as_mut(),
                            futures_async_stream::__reexport::future::get_context(cx),
                        )
                    };
                    dbg!(res);
                    match res {
                        Poll::Ready(_) => $arg = yield Res::Ready,
                        Poll::Pending => $arg = yield Res::Pending,
                    }
                }
            }
        }
    }};
}

async fn bar() {
    let mut first = true;
    future::poll_fn(|cx| {
        if first {
            cx.waker().wake_by_ref();
            first = false;
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    })
    .await;

    eprintln!("after_pending");
}

fn foo<'a, T: std::fmt::Debug, E: std::fmt::Debug + 'a>(
    v: &'a std::sync::Mutex<Vec<T>>,
) -> impl Sink<T, Error = E> + 'a {
    from_generator(static move |mut arg: Arg<T>| -> Result<(), E> {
        while let Some(item) = await_input!(arg) {
            v.lock().unwrap().push(item);

            // dbg!(&item);
            let _i = await_in_sink!(arg, bar());

            eprintln!("continue");
        }

        Ok(())
    })
}

use chan::channel;

mod chan {
    use super::*;
    use futures::lock::Mutex;
    use std::{collections::VecDeque, sync::Arc};

    pub fn channel<T: std::fmt::Debug + Send>(
        cap: usize,
    ) -> (impl Sink<T, Error = Infallible> + Send, impl Stream<Item = T> + Send) {
        let v = Arc::new(Mutex::new(VecDeque::with_capacity(cap)));
        let sender = sender::<T>(v.clone(), cap);
        let receiver = receiver::<T>(v, cap);
        (sender, receiver)
    }

    #[stream(item = T)]
    async fn receiver<T: std::fmt::Debug + Send>(mut v: Arc<Mutex<VecDeque<T>>>, cap: usize) {
        loop {
            if let Some(i) = v.lock().await.pop_front() {
                yield i
            } else {
                let mut first = true;
                future::poll_fn(|cx| {
                    if first {
                        cx.waker().wake_by_ref();
                        first = false;
                        Poll::Pending
                    } else {
                        Poll::Ready(())
                    }
                })
                .await;
            }
        }
    }

    fn sender<T: std::fmt::Debug + Send>(
        v: Arc<Mutex<VecDeque<T>>>,
        cap: usize,
    ) -> impl Sink<T, Error = Infallible> {
        from_generator(static move |mut arg: Arg<T>| -> Result<(), Infallible> {
            while let Some(item) = await_input!(arg) {
                loop {
                    let mut v = await_in_sink!(arg, v.lock());
                    if v.len() < cap {
                        v.push_back(item);
                        break;
                    } else {
                        drop(v);
                        let mut first = true;
                        await_in_sink!(
                            arg,
                            future::poll_fn(|cx| {
                                if first {
                                    cx.waker().wake_by_ref();
                                    first = false;
                                    Poll::Pending
                                } else {
                                    Poll::Ready(())
                                }
                            })
                        );
                    }
                }
            }
            Ok(())
        })
    }
}

#[test]
fn test1() {
    either_sink();
    send();
    send_all();
    mpsc_blocking_start_send();
    with_flush();
    with_as_map();
    with_flat_map();
    with_propagates_poll_ready();
    with_flush_propagate();
    buffer_noop();
    buffer();
    fanout_smoke();
    fanout_backpressure();
    sink_map_err();
}

fn main() {
    channel_test::send_recv();

    // let mut sink = Box::pin(foo::<u32>());
    // let w = noop_waker();
    // let cx = &mut Context::from_waker(&w);
    // let mut rt = tokio::runtime::Runtime::new().unwrap();
    // {
    //     for x in 0..10 {
    //         // run(sink.as_mut().send(x));
    //         dbg!(rt.block_on(sink.send(x)));
    //     }
    //     dbg!(rt.block_on(sink.send_all(&mut stream::iter((1..5).map(Ok).collect::<Vec<_>>()))));
    // }
    // std::thread::spawn(|| {std::thread::sleep(std::time::Duration::from_secs(5); std::thread::un}));
    // dbg!(block_on(sink.as_mut().send(1)));
    // dbg!(block_on(sink.as_mut().send(1)));
    // run(sink.as_mut().send("foo".into()));
    // run(sink.as_mut().send("foo".into()));
    // run(sink.as_mut().send("foo".into()));
    // dbg!(block_on(sink.as_mut().close()));
    // dbg!(sink.as_mut().poll_ready(cx));
    // dbg!(sink.as_mut().start_send("foo".into()));
    // dbg!(sink.as_mut().poll_flush(cx));
    // dbg!(sink.as_mut().poll_flush(cx));
    // dbg!(sink.as_mut().start_send("foo".into()));
    // dbg!(sink.as_mut().poll_close(cx));
    // dbg!(sink.as_mut().poll_ready(cx));
    // dbg!(sink.as_mut().poll_flush(cx));
    // dbg!(sink.as_mut().poll_close(cx));
    // dbg!(sink.as_mut().poll_flush(cx)); // <-- panic
}

fn either_sink() {
    use futures::sink::{Sink, SinkExt};
    use std::{collections::VecDeque, pin::Pin};

    let m = Default::default();
    let mut s = Box::pin(if true {
        foo::<i32, ()>(&m).left_sink()
    } else {
        foo::<i32, ()>(&m).right_sink()
    });

    s.as_mut().start_send(0).unwrap();
    assert_eq!(*m.lock().unwrap(), vec![]);
}

fn send() {
    use futures::{executor::block_on, sink::SinkExt};

    let m = Default::default();
    let mut v = Box::pin(foo::<i32, ()>(&m));

    block_on(v.send(0)).unwrap();
    assert_eq!(*m.lock().unwrap(), vec![0]);

    block_on(v.send(1)).unwrap();
    assert_eq!(*m.lock().unwrap(), vec![0, 1]);

    block_on(v.send(2)).unwrap();
    assert_eq!(*m.lock().unwrap(), vec![0, 1, 2]);
}

fn send_all() {
    use futures::{
        executor::block_on,
        sink::SinkExt,
        stream::{self, StreamExt},
    };

    let m = Default::default();
    let mut v = Box::pin(foo::<i32, ()>(&m));

    block_on(v.send_all(&mut stream::iter(vec![0, 1]).map(Ok))).unwrap();
    assert_eq!(*m.lock().unwrap(), vec![0, 1]);

    block_on(v.send_all(&mut stream::iter(vec![2, 3]).map(Ok))).unwrap();
    assert_eq!(*m.lock().unwrap(), vec![0, 1, 2, 3]);

    block_on(v.send_all(&mut stream::iter(vec![4, 5]).map(Ok))).unwrap();
    assert_eq!(*m.lock().unwrap(), vec![0, 1, 2, 3, 4, 5]);
}

// Test that `start_send` on an `mpsc` channel does indeed block when the
// channel is full
fn mpsc_blocking_start_send() {
    use futures::{
        channel::mpsc,
        executor::block_on,
        future::{self, FutureExt},
    };

    use flag_cx::flag_cx;
    use sassert_next::sassert_next;
    use start_send_fut::StartSendFut;
    use unwrap::unwrap;

    let (tx, rx) = channel::<i32>(0);
    pin_mut!(tx, rx);

    block_on(future::lazy(|_| {
        tx.as_mut().start_send(0).unwrap();

        flag_cx(|flag, cx| {
            let mut task = StartSendFut::new(tx, 1);

            assert!(task.poll_unpin(cx).is_pending());
            assert!(!flag.take());
            sassert_next(&mut rx, 0);
            assert!(flag.take());
            unwrap(task.poll_unpin(cx));
            assert!(!flag.take());
            sassert_next(&mut rx, 1);
        })
    }));
}

// test `flush` by using `with` to make the first insertion into a sink block
// until a oneshot is completed
fn with_flush() {
    use futures::{
        channel::oneshot,
        executor::block_on,
        future::{self, FutureExt, TryFutureExt},
        never::Never,
        sink::{Sink, SinkExt},
    };
    use std::{mem, pin::Pin};

    use flag_cx::flag_cx;
    use unwrap::unwrap;

    let (tx, rx) = oneshot::channel();
    let mut block = rx.boxed();
    let m = Default::default();
    let mut sink = Box::pin(foo::<i32, Infallible>(&m)).with(|elem| {
        mem::replace(&mut block, future::ok(()).boxed())
            .map_ok(move |()| elem + 1)
            .map_err(|_| -> Never { panic!() })
    });

    assert_eq!(Pin::new(&mut sink).start_send(0).ok(), Some(()));

    flag_cx(|flag, cx| {
        let mut task = sink.flush();
        assert!(task.poll_unpin(cx).is_pending());
        tx.send(()).unwrap();
        assert!(flag.take());

        assert!(task.poll_unpin(cx).is_pending());
        unwrap(task.poll_unpin(cx));

        block_on(sink.send(1)).unwrap();
        assert_eq!(&*m.lock().unwrap(), &[1, 2]);
    })
}

// test simple use of with to change data
fn with_as_map() {
    use futures::{executor::block_on, future, never::Never, sink::SinkExt};

    let m = Default::default();
    let mut sink =
        Box::pin(foo::<i32, Infallible>(&m)).with(|item| future::ok::<i32, Never>(item * 2));
    block_on(sink.send(0)).unwrap();
    block_on(sink.send(1)).unwrap();
    block_on(sink.send(2)).unwrap();
    assert_eq!(&*m.lock().unwrap(), &[0, 2, 4]);
}

// test simple use of with_flat_map
fn with_flat_map() {
    use futures::{
        executor::block_on,
        sink::SinkExt,
        stream::{self, StreamExt},
    };

    let m = Default::default();
    let mut sink = Box::pin(foo::<usize, Infallible>(&m))
        .with_flat_map(|item| stream::iter(vec![item; item]).map(Ok));
    block_on(sink.send(0)).unwrap();
    block_on(sink.send(1)).unwrap();
    block_on(sink.send(2)).unwrap();
    block_on(sink.send(3)).unwrap();
    assert_eq!(&*m.lock().unwrap(), &[1, 2, 2, 3, 3, 3]);
}

// Check that `with` propagates `poll_ready` to the inner sink.
// Regression test for the issue #1834.
fn with_propagates_poll_ready() {
    use futures::{
        channel::mpsc,
        executor::block_on,
        future,
        sink::{Sink, SinkExt},
        task::Poll,
    };
    use std::pin::Pin;

    use flag_cx::flag_cx;
    use sassert_next::sassert_next;

    let (tx, rx) = channel::<i32>(0);
    pin_mut!(tx, rx);
    let mut tx = tx.with(|item: i32| future::ok::<i32, Infallible>(item + 10));

    block_on(future::lazy(|_| {
        flag_cx(|flag, cx| {
            let mut tx = Pin::new(&mut tx);

            // Should be ready for the first item.
            assert_eq!(tx.as_mut().poll_ready(cx), Poll::Ready(Ok(())));
            assert_eq!(tx.as_mut().start_send(0), Ok(()));

            // Should be ready for the second item only after the first one is received.
            assert_eq!(tx.as_mut().poll_ready(cx), Poll::Pending);
            assert!(!flag.take());
            sassert_next(&mut rx, 10);
            assert!(flag.take());
            assert_eq!(tx.as_mut().poll_ready(cx), Poll::Ready(Ok(())));
            assert_eq!(tx.as_mut().start_send(1), Ok(()));
        })
    }));
}

// test that the `with` sink doesn't require the underlying sink to flush,
// but doesn't claim to be flushed until the underlying sink is
fn with_flush_propagate() {
    use futures::{
        future::{self, FutureExt},
        sink::{Sink, SinkExt},
    };
    use std::pin::Pin;

    use flag_cx::flag_cx;
    use manual_flush::ManualFlush;
    use unwrap::unwrap;

    let mut sink = ManualFlush::new().with(future::ok::<Option<i32>, ()>);
    flag_cx(|flag, cx| {
        unwrap(Pin::new(&mut sink).poll_ready(cx));
        Pin::new(&mut sink).start_send(Some(0)).unwrap();
        unwrap(Pin::new(&mut sink).poll_ready(cx));
        Pin::new(&mut sink).start_send(Some(1)).unwrap();

        {
            let mut task = sink.flush();
            assert!(task.poll_unpin(cx).is_pending());
            assert!(!flag.take());
        }
        assert_eq!(sink.get_mut().force_flush(), vec![0, 1]);
        assert!(flag.take());
        unwrap(sink.flush().poll_unpin(cx));
    })
}

// test that a buffer is a no-nop around a sink that always accepts sends
fn buffer_noop() {
    use futures::{executor::block_on, sink::SinkExt};

    let m = Default::default();
    let mut sink = Box::pin(foo::<usize, Infallible>(&m)).buffer(0);
    block_on(sink.send(0)).unwrap();
    block_on(sink.send(1)).unwrap();
    assert_eq!(&*m.lock().unwrap(), &[0, 1]);

    let m = Default::default();
    let mut sink = Box::pin(foo::<usize, Infallible>(&m)).buffer(1);
    block_on(sink.send(0)).unwrap();
    block_on(sink.send(1)).unwrap();
    assert_eq!(&*m.lock().unwrap(), &[0, 1]);
}

// test basic buffer functionality, including both filling up to capacity,
// and writing out when the underlying sink is ready
fn buffer() {
    use futures::{executor::block_on, future::FutureExt, sink::SinkExt};

    use allowance::manual_allow;
    use flag_cx::flag_cx;
    use start_send_fut::StartSendFut;
    use unwrap::unwrap;

    let (sink, allow) = manual_allow::<i32>();
    let sink = sink.buffer(2);

    let sink = block_on(StartSendFut::new(sink, 0)).unwrap();
    let mut sink = block_on(StartSendFut::new(sink, 1)).unwrap();

    flag_cx(|flag, cx| {
        let mut task = sink.send(2);
        assert!(task.poll_unpin(cx).is_pending());
        assert!(!flag.take());
        allow.start();
        assert!(flag.take());
        unwrap(task.poll_unpin(cx));
        assert_eq!(sink.get_ref().data, vec![0, 1, 2]);
    })
}

fn fanout_smoke() {
    use futures::{
        executor::block_on,
        sink::SinkExt,
        stream::{self, StreamExt},
    };

    let m1 = Default::default();
    let m2 = Default::default();
    let sink1 = Box::pin(foo::<usize, Infallible>(&m1));
    let sink2 = Box::pin(foo::<usize, Infallible>(&m2));
    let mut sink = sink1.fanout(sink2);
    block_on(sink.send_all(&mut stream::iter(vec![1, 2, 3]).map(Ok))).unwrap();
    let (sink1, sink2) = sink.into_inner();
    assert_eq!(&*m1.lock().unwrap(), &[1, 2, 3]);
    assert_eq!(&*m2.lock().unwrap(), &[1, 2, 3]);
}

fn fanout_backpressure() {
    use futures::{
        channel::mpsc, executor::block_on, future::FutureExt, sink::SinkExt, stream::StreamExt,
    };

    use flag_cx::flag_cx;
    use start_send_fut::StartSendFut;
    use unwrap::unwrap;

    let (left_send, left_recv) = channel::<i32>(0);
    pin_mut!(left_send, left_recv);
    let (right_send, right_recv) = channel::<i32>(0);
    pin_mut!(right_send, right_recv);
    let sink = left_send.fanout(right_send);

    let mut sink = block_on(StartSendFut::new(sink, 0)).unwrap();

    flag_cx(|flag, cx| {
        let mut task = sink.send(2);
        assert!(!flag.take());
        assert!(task.poll_unpin(cx).is_pending());
        assert_eq!(block_on(left_recv.next()), Some(0));
        assert!(flag.take());
        assert!(task.poll_unpin(cx).is_pending());
        assert_eq!(block_on(right_recv.next()), Some(0));
        assert!(flag.take());

        assert!(task.poll_unpin(cx).is_pending());
        assert_eq!(block_on(left_recv.next()), Some(2));
        assert!(flag.take());
        assert!(task.poll_unpin(cx).is_pending());
        assert_eq!(block_on(right_recv.next()), Some(2));
        assert!(flag.take());

        unwrap(task.poll_unpin(cx));
        // make sure receivers live until end of test to prevent send errors
        drop(left_recv);
        drop(right_recv);
    })
}

fn sink_map_err() {
    use futures::{
        channel::mpsc,
        sink::{Sink, SinkExt},
        task::Poll,
    };
    use futures_test::task::panic_context;
    use std::pin::Pin;

    {
        let cx = &mut panic_context();
        let (tx, _rx) = channel::<i32>(1);
        pin_mut!(tx, _rx);

        let mut tx = tx.sink_map_err(|_| ());
        assert_eq!(Pin::new(&mut tx).start_send(0), Ok(()));
        assert_eq!(Pin::new(&mut tx).poll_flush(cx), Poll::Ready(Ok(())));
    }

    // let tx = mpsc::channel(0).0;
    // assert_eq!(Pin::new(&mut tx.sink_map_err(|_| ())).start_send(0), Err(()));
}

mod sassert_next {
    use futures::{
        stream::{Stream, StreamExt},
        task::Poll,
    };
    use futures_test::task::panic_context;
    use std::fmt;

    pub fn sassert_next<S>(s: &mut S, item: S::Item)
    where
        S: Stream + Unpin,
        S::Item: Eq + fmt::Debug,
    {
        match s.poll_next_unpin(&mut panic_context()) {
            Poll::Ready(None) => panic!("stream is at its end"),
            Poll::Ready(Some(e)) => assert_eq!(e, item),
            Poll::Pending => panic!("stream wasn't ready"),
        }
    }
}

mod unwrap {
    use futures::task::Poll;
    use std::fmt;

    pub fn unwrap<T, E: fmt::Debug>(x: Poll<Result<T, E>>) -> T {
        match x {
            Poll::Ready(Ok(x)) => x,
            Poll::Ready(Err(_)) => panic!("Poll::Ready(Err(_))"),
            Poll::Pending => panic!("Poll::Pending"),
        }
    }
}

mod flag_cx {
    use futures::task::{self, ArcWake, Context};
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    // An Unpark struct that records unpark events for inspection
    pub struct Flag(AtomicBool);

    impl Flag {
        pub fn new() -> Arc<Self> {
            Arc::new(Self(AtomicBool::new(false)))
        }

        pub fn take(&self) -> bool {
            self.0.swap(false, Ordering::SeqCst)
        }

        pub fn set(&self, v: bool) {
            self.0.store(v, Ordering::SeqCst)
        }
    }

    impl ArcWake for Flag {
        fn wake_by_ref(arc_self: &Arc<Self>) {
            arc_self.set(true)
        }
    }

    pub fn flag_cx<F, R>(f: F) -> R
    where
        F: FnOnce(Arc<Flag>, &mut Context<'_>) -> R,
    {
        let flag = Flag::new();
        let waker = task::waker_ref(&flag);
        let cx = &mut Context::from_waker(&waker);
        f(flag.clone(), cx)
    }
}

mod start_send_fut {
    use futures::{
        future::Future,
        ready,
        sink::Sink,
        task::{Context, Poll},
    };
    use std::pin::Pin;

    // Sends a value on an i32 channel sink
    pub struct StartSendFut<S: Sink<Item> + Unpin, Item: Unpin>(Option<S>, Option<Item>);

    impl<S: Sink<Item> + Unpin, Item: Unpin> StartSendFut<S, Item> {
        pub fn new(sink: S, item: Item) -> Self {
            Self(Some(sink), Some(item))
        }
    }

    impl<S: Sink<Item> + Unpin, Item: Unpin> Future for StartSendFut<S, Item> {
        type Output = Result<S, S::Error>;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let Self(inner, item) = self.get_mut();
            {
                let mut inner = inner.as_mut().unwrap();
                ready!(Pin::new(&mut inner).poll_ready(cx))?;
                Pin::new(&mut inner).start_send(item.take().unwrap())?;
            }
            Poll::Ready(Ok(inner.take().unwrap()))
        }
    }
}

mod manual_flush {
    use futures::{
        sink::Sink,
        task::{Context, Poll, Waker},
    };
    use std::{mem, pin::Pin};

    // Immediately accepts all requests to start pushing, but completion is managed
    // by manually flushing
    pub struct ManualFlush<T: Unpin> {
        data: Vec<T>,
        waiting_tasks: Vec<Waker>,
    }

    impl<T: Unpin> Sink<Option<T>> for ManualFlush<T> {
        type Error = ();

        fn poll_ready(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn start_send(mut self: Pin<&mut Self>, item: Option<T>) -> Result<(), Self::Error> {
            if let Some(item) = item {
                self.data.push(item);
            } else {
                self.force_flush();
            }
            Ok(())
        }

        fn poll_flush(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            if self.data.is_empty() {
                Poll::Ready(Ok(()))
            } else {
                self.waiting_tasks.push(cx.waker().clone());
                Poll::Pending
            }
        }

        fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            self.poll_flush(cx)
        }
    }

    impl<T: Unpin> ManualFlush<T> {
        pub fn new() -> Self {
            Self { data: Vec::new(), waiting_tasks: Vec::new() }
        }

        pub fn force_flush(&mut self) -> Vec<T> {
            for task in self.waiting_tasks.drain(..) {
                task.wake()
            }
            mem::replace(&mut self.data, Vec::new())
        }
    }
}

mod allowance {
    use futures::{
        sink::Sink,
        task::{Context, Poll, Waker},
    };
    use std::{
        cell::{Cell, RefCell},
        pin::Pin,
        rc::Rc,
    };

    pub struct ManualAllow<T: Unpin> {
        pub data: Vec<T>,
        allow: Rc<Allow>,
    }

    pub struct Allow {
        flag: Cell<bool>,
        tasks: RefCell<Vec<Waker>>,
    }

    impl Allow {
        pub fn new() -> Self {
            Self { flag: Cell::new(false), tasks: RefCell::new(Vec::new()) }
        }

        pub fn check(&self, cx: &mut Context<'_>) -> bool {
            if self.flag.get() {
                true
            } else {
                self.tasks.borrow_mut().push(cx.waker().clone());
                false
            }
        }

        pub fn start(&self) {
            self.flag.set(true);
            let mut tasks = self.tasks.borrow_mut();
            for task in tasks.drain(..) {
                task.wake();
            }
        }
    }

    impl<T: Unpin> Sink<T> for ManualAllow<T> {
        type Error = ();

        fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            if self.allow.check(cx) { Poll::Ready(Ok(())) } else { Poll::Pending }
        }

        fn start_send(mut self: Pin<&mut Self>, item: T) -> Result<(), Self::Error> {
            self.data.push(item);
            Ok(())
        }

        fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
    }

    pub fn manual_allow<T: Unpin>() -> (ManualAllow<T>, Rc<Allow>) {
        let allow = Rc::new(Allow::new());
        let manual_allow = ManualAllow { data: Vec::new(), allow: allow.clone() };
        (manual_allow, allow)
    }
}

mod channel_test {
    use super::*;

    use futures::{
        channel::{mpsc, oneshot},
        executor::{block_on, block_on_stream},
        future::{poll_fn, FutureExt},
        pin_mut,
        sink::{Sink, SinkExt},
        stream::{Stream, StreamExt},
        task::{Context, Poll},
    };
    use futures_test::task::{new_count_waker, noop_context};
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc, Mutex,
        },
        thread,
    };

    // #[test]
    pub fn send_recv() {
        let (tx, rx) = channel::<i32>(16);
        pin_mut!(tx, rx);

        block_on(tx.send(1)).unwrap();
        // drop(tx);
        let v: Vec<_> = block_on(rx.collect());
        assert_eq!(v, vec![1]);
    }

    #[test]
    fn send_recv_no_buffer() {
        // Run on a task context
        block_on(poll_fn(move |cx| {
            let (tx, rx) = channel::<i32>(0);
            pin_mut!(tx, rx);

            assert!(tx.as_mut().poll_flush(cx).is_ready());
            assert!(tx.as_mut().poll_ready(cx).is_ready());

            // Send first message
            assert!(tx.as_mut().start_send(1).is_ok());
            assert!(tx.as_mut().poll_ready(cx).is_pending());

            // poll_ready said Pending, so no room in buffer, therefore new sends
            // should get rejected with is_full.
            tx.as_mut().start_send(0);
            // assert!(tx.as_mut().start_send(0).unwrap_err().is_full());
            assert!(tx.as_mut().poll_ready(cx).is_pending());

            // Take the value
            assert_eq!(rx.as_mut().poll_next(cx), Poll::Ready(Some(1)));
            assert!(tx.as_mut().poll_ready(cx).is_ready());

            // Send second message
            assert!(tx.as_mut().poll_ready(cx).is_ready());
            assert!(tx.as_mut().start_send(2).is_ok());
            assert!(tx.as_mut().poll_ready(cx).is_pending());

            // Take the value
            assert_eq!(rx.as_mut().poll_next(cx), Poll::Ready(Some(2)));
            assert!(tx.as_mut().poll_ready(cx).is_ready());

            Poll::Ready(())
        }));
    }

    // #[test]
    // fn send_shared_recv() {
    //     let (mut tx1, rx) = mpsc::channel::<i32>(16);
    //     let mut rx = block_on_stream(rx);
    //     let mut tx2 = tx1.clone();

    //     block_on(tx1.send(1)).unwrap();
    //     assert_eq!(rx.next(), Some(1));

    //     block_on(tx2.send(2)).unwrap();
    //     assert_eq!(rx.next(), Some(2));
    // }

    #[ignore]
    #[test]
    fn send_recv_threads() {
        let (tx, rx) = channel::<i32>(16);
        let (mut tx, mut rx) = (Box::pin(tx), Box::pin(rx));

        let t = thread::spawn(move || {
            block_on(tx.send(1)).unwrap();
        });

        let v: Vec<_> = block_on(rx.take(1).collect());
        assert_eq!(v, vec![1]);

        t.join().unwrap();
    }

    #[test]
    fn send_recv_threads_no_capacity() {
        let (tx, rx) = mpsc::channel::<i32>(0);
        let (mut tx, mut rx) = (Box::pin(tx), Box::pin(rx));

        let t = thread::spawn(move || {
            block_on(tx.send(1)).unwrap();
            block_on(tx.send(2)).unwrap();
        });

        let v: Vec<_> = block_on(rx.collect());
        assert_eq!(v, vec![1, 2]);

        t.join().unwrap();
    }

    // #[test]
    // fn recv_close_gets_none() {
    //     let (tx, rx) = mpsc::channel::<i32>(10);
    //     pin_mut!(tx, rx);

    //     // Run on a task context
    //     block_on(poll_fn(move |cx| {
    //         rx.close();

    //         assert_eq!(rx.poll_next_unpin(cx), Poll::Ready(None));
    //         match tx.poll_ready(cx) {
    //             Poll::Pending | Poll::Ready(Ok(_)) => panic!(),
    //             Poll::Ready(Err(e)) => assert!(e.is_disconnected()),
    //         };

    //         Poll::Ready(())
    //     }));
    // }

    #[test]
    fn tx_close_gets_none() {
        let (_, rx) = mpsc::channel::<i32>(10);
        pin_mut!(rx);

        // Run on a task context
        block_on(poll_fn(move |cx| {
            assert_eq!(rx.poll_next_unpin(cx), Poll::Ready(None));
            Poll::Ready(())
        }));
    }

    // #[test]
    // fn spawn_sends_items() {
    //     let core = local_executor::Core::new();
    //     let stream = unfold(0, |i| Some(ok::<_,u8>((i, i + 1))));
    //     let rx = mpsc::spawn(stream, &core, 1);
    //     assert_eq!(core.run(rx.take(4).collect()).unwrap(),
    //                [0, 1, 2, 3]);
    // }

    // #[test]
    // fn spawn_kill_dead_stream() {
    //     use std::thread;
    //     use std::time::Duration;
    //     use futures::future::Either;
    //     use futures::sync::oneshot;
    //
    //     // a stream which never returns anything (maybe a remote end isn't
    //     // responding), but dropping it leads to observable side effects
    //     // (like closing connections, releasing limited resources, ...)
    //     #[derive(Debug)]
    //     struct Dead {
    //         // when dropped you should get Err(oneshot::Canceled) on the
    //         // receiving end
    //         done: oneshot::Sender<()>,
    //     }
    //     impl Stream for Dead {
    //         type Item = ();
    //         type Error = ();
    //
    //         fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
    //             Ok(Poll::Pending)
    //         }
    //     }
    //
    //     // need to implement a timeout for the test, as it would hang
    //     // forever right now
    //     let (timeout_tx, timeout_rx) = oneshot::channel();
    //     thread::spawn(move || {
    //         thread::sleep(Duration::from_millis(1000));
    //         let _ = timeout_tx.send(());
    //     });
    //
    //     let core = local_executor::Core::new();
    //     let (done_tx, done_rx) = oneshot::channel();
    //     let stream = Dead{done: done_tx};
    //     let rx = mpsc::spawn(stream, &core, 1);
    //     let res = core.run(
    //         Ok::<_, ()>(())
    //         .into_future()
    //         .then(move |_| {
    //             // now drop the spawned stream: maybe some timeout exceeded,
    //             // or some connection on this end was closed by the remote
    //             // end.
    //             drop(rx);
    //             // and wait for the spawned stream to release its resources
    //             done_rx
    //         })
    //         .select2(timeout_rx)
    //     );
    //     match res {
    //         Err(Either::A((oneshot::Canceled, _))) => (),
    //         _ => {
    //             panic!("dead stream wasn't canceled");
    //         },
    //     }
    // }

    #[test]
    fn stress_shared_unbounded() {
        const AMT: u32 = 10000;
        const NTHREADS: u32 = 8;
        let (tx, rx) = mpsc::unbounded::<i32>();

        let t = thread::spawn(move || {
            let result: Vec<_> = block_on(rx.collect());
            assert_eq!(result.len(), (AMT * NTHREADS) as usize);
            for item in result {
                assert_eq!(item, 1);
            }
        });

        for _ in 0..NTHREADS {
            let tx = tx.clone();

            thread::spawn(move || {
                for _ in 0..AMT {
                    tx.unbounded_send(1).unwrap();
                }
            });
        }

        drop(tx);

        t.join().ok().unwrap();
    }

    #[test]
    fn stress_shared_bounded_hard() {
        const AMT: u32 = 10000;
        const NTHREADS: u32 = 8;
        let (tx, rx) = mpsc::channel::<i32>(0);

        let t = thread::spawn(move || {
            let result: Vec<_> = block_on(rx.collect());
            assert_eq!(result.len(), (AMT * NTHREADS) as usize);
            for item in result {
                assert_eq!(item, 1);
            }
        });

        for _ in 0..NTHREADS {
            let mut tx = tx.clone();

            thread::spawn(move || {
                for _ in 0..AMT {
                    block_on(tx.send(1)).unwrap();
                }
            });
        }

        drop(tx);

        t.join().unwrap();
    }

    #[test]
    fn stress_receiver_multi_task_bounded_hard() {
        const AMT: usize = 10_000;
        const NTHREADS: u32 = 2;

        let (mut tx, rx) = mpsc::channel::<usize>(0);
        let rx = Arc::new(Mutex::new(Some(rx)));
        let n = Arc::new(AtomicUsize::new(0));

        let mut th = vec![];

        for _ in 0..NTHREADS {
            let rx = rx.clone();
            let n = n.clone();

            let t = thread::spawn(move || {
                let mut i = 0;

                loop {
                    i += 1;
                    let mut rx_opt = rx.lock().unwrap();
                    if let Some(rx) = &mut *rx_opt {
                        if i % 5 == 0 {
                            let item = block_on(rx.next());

                            if item.is_none() {
                                *rx_opt = None;
                                break;
                            }

                            n.fetch_add(1, Ordering::Relaxed);
                        } else {
                            // Just poll
                            let n = n.clone();
                            match rx.poll_next_unpin(&mut noop_context()) {
                                Poll::Ready(Some(_)) => {
                                    n.fetch_add(1, Ordering::Relaxed);
                                }
                                Poll::Ready(None) => {
                                    *rx_opt = None;
                                    break;
                                }
                                Poll::Pending => {}
                            }
                        }
                    } else {
                        break;
                    }
                }
            });

            th.push(t);
        }

        for i in 0..AMT {
            block_on(tx.send(i)).unwrap();
        }
        drop(tx);

        for t in th {
            t.join().unwrap();
        }

        assert_eq!(AMT, n.load(Ordering::Relaxed));
    }

    /// Stress test that receiver properly receives all the messages
    /// after sender dropped.
    #[test]
    fn stress_drop_sender() {
        fn list() -> impl Stream<Item = i32> {
            let (tx, rx) = mpsc::channel(1);
            thread::spawn(move || {
                block_on(send_one_two_three(tx));
            });
            rx
        }

        for _ in 0..10000 {
            let v: Vec<_> = block_on(list().collect());
            assert_eq!(v, vec![1, 2, 3]);
        }
    }

    async fn send_one_two_three(mut tx: mpsc::Sender<i32>) {
        for i in 1..=3 {
            tx.send(i).await.unwrap();
        }
    }

    /// Stress test that after receiver dropped,
    /// no messages are lost.
    fn stress_close_receiver_iter() {
        let (tx, rx) = mpsc::unbounded();
        let mut rx = block_on_stream(rx);
        let (unwritten_tx, unwritten_rx) = std::sync::mpsc::channel();
        let th = thread::spawn(move || {
            for i in 1.. {
                if tx.unbounded_send(i).is_err() {
                    unwritten_tx.send(i).expect("unwritten_tx");
                    return;
                }
            }
        });

        // Read one message to make sure thread effectively started
        assert_eq!(Some(1), rx.next());

        rx.close();

        for i in 2.. {
            match rx.next() {
                Some(r) => assert!(i == r),
                None => {
                    let unwritten = unwritten_rx.recv().expect("unwritten_rx");
                    assert_eq!(unwritten, i);
                    th.join().unwrap();
                    return;
                }
            }
        }
    }

    #[test]
    fn stress_close_receiver() {
        for _ in 0..10000 {
            stress_close_receiver_iter();
        }
    }

    async fn stress_poll_ready_sender(mut sender: mpsc::Sender<u32>, count: u32) {
        for i in (1..=count).rev() {
            sender.send(i).await.unwrap();
        }
    }

    /// Tests that after `poll_ready` indicates capacity a channel can always send without waiting.
    #[test]
    fn stress_poll_ready() {
        const AMT: u32 = 1000;
        const NTHREADS: u32 = 8;

        /// Run a stress test using the specified channel capacity.
        fn stress(capacity: usize) {
            let (tx, rx) = mpsc::channel(capacity);
            let mut threads = Vec::new();
            for _ in 0..NTHREADS {
                let sender = tx.clone();
                threads
                    .push(thread::spawn(move || block_on(stress_poll_ready_sender(sender, AMT))));
            }
            drop(tx);

            let result: Vec<_> = block_on(rx.collect());
            assert_eq!(result.len() as u32, AMT * NTHREADS);

            for thread in threads {
                thread.join().unwrap();
            }
        }

        stress(0);
        stress(1);
        stress(8);
        stress(16);
    }

    #[test]
    fn try_send_1() {
        const N: usize = 3000;
        let (mut tx, rx) = mpsc::channel(0);

        let t = thread::spawn(move || {
            for i in 0..N {
                loop {
                    if tx.try_send(i).is_ok() {
                        break;
                    }
                }
            }
        });

        let result: Vec<_> = block_on(rx.collect());
        for (i, j) in result.into_iter().enumerate() {
            assert_eq!(i, j);
        }

        t.join().unwrap();
    }

    #[test]
    fn try_send_2() {
        let (mut tx, rx) = mpsc::channel(0);
        let mut rx = block_on_stream(rx);

        tx.try_send("hello").unwrap();

        let (readytx, readyrx) = oneshot::channel::<()>();

        let th = thread::spawn(move || {
            block_on(poll_fn(|cx| {
                assert!(tx.poll_ready(cx).is_pending());
                Poll::Ready(())
            }));

            drop(readytx);
            block_on(tx.send("goodbye")).unwrap();
        });

        let _ = block_on(readyrx);
        assert_eq!(rx.next(), Some("hello"));
        assert_eq!(rx.next(), Some("goodbye"));
        assert_eq!(rx.next(), None);

        th.join().unwrap();
    }

    #[test]
    fn try_send_fail() {
        let (mut tx, rx) = mpsc::channel(0);
        let mut rx = block_on_stream(rx);

        tx.try_send("hello").unwrap();

        // This should fail
        assert!(tx.try_send("fail").is_err());

        assert_eq!(rx.next(), Some("hello"));

        tx.try_send("goodbye").unwrap();
        drop(tx);

        assert_eq!(rx.next(), Some("goodbye"));
        assert_eq!(rx.next(), None);
    }

    #[test]
    fn try_send_recv() {
        let (mut tx, mut rx) = mpsc::channel(1);
        tx.try_send("hello").unwrap();
        tx.try_send("hello").unwrap();
        tx.try_send("hello").unwrap_err(); // should be full
        rx.try_next().unwrap();
        rx.try_next().unwrap();
        rx.try_next().unwrap_err(); // should be empty
        tx.try_send("hello").unwrap();
        rx.try_next().unwrap();
        rx.try_next().unwrap_err(); // should be empty
    }

    #[test]
    fn same_receiver() {
        let (mut txa1, _) = mpsc::channel::<i32>(1);
        let txa2 = txa1.clone();

        let (mut txb1, _) = mpsc::channel::<i32>(1);
        let txb2 = txb1.clone();

        assert!(txa1.same_receiver(&txa2));
        assert!(txb1.same_receiver(&txb2));
        assert!(!txa1.same_receiver(&txb1));

        txa1.disconnect();
        txb1.close_channel();

        assert!(!txa1.same_receiver(&txa2));
        assert!(txb1.same_receiver(&txb2));
    }

    #[test]
    fn hash_receiver() {
        use std::{collections::hash_map::DefaultHasher, hash::Hasher};

        let mut hasher_a1 = DefaultHasher::new();
        let mut hasher_a2 = DefaultHasher::new();
        let mut hasher_b1 = DefaultHasher::new();
        let mut hasher_b2 = DefaultHasher::new();
        let (mut txa1, _) = mpsc::channel::<i32>(1);
        let txa2 = txa1.clone();

        let (mut txb1, _) = mpsc::channel::<i32>(1);
        let txb2 = txb1.clone();

        txa1.hash_receiver(&mut hasher_a1);
        let hash_a1 = hasher_a1.finish();
        txa2.hash_receiver(&mut hasher_a2);
        let hash_a2 = hasher_a2.finish();
        txb1.hash_receiver(&mut hasher_b1);
        let hash_b1 = hasher_b1.finish();
        txb2.hash_receiver(&mut hasher_b2);
        let hash_b2 = hasher_b2.finish();

        assert_eq!(hash_a1, hash_a2);
        assert_eq!(hash_b1, hash_b2);
        assert!(hash_a1 != hash_b1);

        txa1.disconnect();
        txb1.close_channel();

        let mut hasher_a1 = DefaultHasher::new();
        let mut hasher_a2 = DefaultHasher::new();
        let mut hasher_b1 = DefaultHasher::new();
        let mut hasher_b2 = DefaultHasher::new();

        txa1.hash_receiver(&mut hasher_a1);
        let hash_a1 = hasher_a1.finish();
        txa2.hash_receiver(&mut hasher_a2);
        let hash_a2 = hasher_a2.finish();
        txb1.hash_receiver(&mut hasher_b1);
        let hash_b1 = hasher_b1.finish();
        txb2.hash_receiver(&mut hasher_b2);
        let hash_b2 = hasher_b2.finish();

        assert!(hash_a1 != hash_a2);
        assert_eq!(hash_b1, hash_b2);
    }

    #[test]
    fn send_backpressure() {
        let (waker, counter) = new_count_waker();
        let mut cx = Context::from_waker(&waker);

        let (mut tx, mut rx) = mpsc::channel(1);
        block_on(tx.send(1)).unwrap();

        let mut task = tx.send(2);
        assert_eq!(task.poll_unpin(&mut cx), Poll::Pending);
        assert_eq!(counter, 0);

        let item = block_on(rx.next()).unwrap();
        assert_eq!(item, 1);
        assert_eq!(counter, 1);
        assert_eq!(task.poll_unpin(&mut cx), Poll::Ready(Ok(())));

        let item = block_on(rx.next()).unwrap();
        assert_eq!(item, 2);
    }

    #[test]
    fn send_backpressure_multi_senders() {
        let (waker, counter) = new_count_waker();
        let mut cx = Context::from_waker(&waker);

        let (mut tx1, mut rx) = mpsc::channel(1);
        let mut tx2 = tx1.clone();
        block_on(tx1.send(1)).unwrap();

        let mut task = tx2.send(2);
        assert_eq!(task.poll_unpin(&mut cx), Poll::Pending);
        assert_eq!(counter, 0);

        let item = block_on(rx.next()).unwrap();
        assert_eq!(item, 1);
        assert_eq!(counter, 1);
        assert_eq!(task.poll_unpin(&mut cx), Poll::Ready(Ok(())));

        let item = block_on(rx.next()).unwrap();
        assert_eq!(item, 2);
    }
}
