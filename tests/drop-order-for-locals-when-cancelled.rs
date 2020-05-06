// edition:2018
// run-pass

#![allow(unused_variables, unused_must_use, path_statements)]
#![deny(dead_code)]
#![feature(generators)]

// Test that the drop order for locals in a fn and async fn matches up.

use futures::{
    stream::Stream,
    task::{self, ArcWake},
};
use futures_async_stream::stream;
use std::{
    cell::RefCell,
    future::Future,
    pin::Pin,
    rc::Rc,
    sync::Arc,
    task::{Context, Poll},
};

struct EmptyWaker;

impl ArcWake for EmptyWaker {
    fn wake_by_ref(arc_self: &Arc<Self>) {}
}

#[derive(Debug, Eq, PartialEq)]
enum DropOrder {
    Function,
    Val(&'static str),
}

type DropOrderListPtr = Rc<RefCell<Vec<DropOrder>>>;

struct D(&'static str, DropOrderListPtr);

impl Drop for D {
    fn drop(&mut self) {
        self.1.borrow_mut().push(DropOrder::Val(self.0));
    }
}

struct NeverReady;

impl Future for NeverReady {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        Poll::Pending
    }
}

#[stream(item = ())]
async fn simple_variable_declaration_async(l: DropOrderListPtr) {
    l.borrow_mut().push(DropOrder::Function);
    let x = D("x", l.clone());
    let y = D("y", l.clone());
    NeverReady.await;
}

fn simple_variable_declaration_sync(l: DropOrderListPtr) {
    l.borrow_mut().push(DropOrder::Function);
    let x = D("x", l.clone());
    let y = D("y", l.clone());
}

#[stream(item = ())]
async fn varable_completely_contained_within_block_async(l: DropOrderListPtr) {
    l.borrow_mut().push(DropOrder::Function);
    async {
        let x = D("x", l.clone());
    }
    .await;
    let y = D("y", l.clone());
    NeverReady.await;
}

fn varable_completely_contained_within_block_sync(l: DropOrderListPtr) {
    l.borrow_mut().push(DropOrder::Function);
    {
        let x = D("x", l.clone());
    }
    let y = D("y", l.clone());
}

#[stream(item = ())]
async fn variables_moved_into_separate_blocks_async(l: DropOrderListPtr) {
    l.borrow_mut().push(DropOrder::Function);
    let x = D("x", l.clone());
    let y = D("y", l.clone());
    async move { x }.await;
    async move { y }.await;
    NeverReady.await;
}

fn variables_moved_into_separate_blocks_sync(l: DropOrderListPtr) {
    l.borrow_mut().push(DropOrder::Function);
    let x = D("x", l.clone());
    let y = D("y", l.clone());
    {
        x
    };
    {
        y
    };
}

#[stream(item = ())]
async fn variables_moved_into_same_block_async(l: DropOrderListPtr) {
    l.borrow_mut().push(DropOrder::Function);
    let x = D("x", l.clone());
    let y = D("y", l.clone());
    async move {
        x;
        y;
    };
    NeverReady.await;
}

fn variables_moved_into_same_block_sync(l: DropOrderListPtr) {
    l.borrow_mut().push(DropOrder::Function);
    let x = D("x", l.clone());
    let y = D("y", l.clone());
    {
        x;
        y;
    };
    return;
}

#[stream(item = ())]
async fn move_after_current_await_doesnt_affect_order(l: DropOrderListPtr) {
    l.borrow_mut().push(DropOrder::Function);
    let x = D("x", l.clone());
    let y = D("y", l.clone());
    NeverReady.await;
    async move {
        x;
        y;
    };
}

fn assert_drop_order_after_cancel<St: Stream<Item = ()>>(
    f: impl FnOnce(DropOrderListPtr) -> St,
    g: impl FnOnce(DropOrderListPtr),
) {
    let empty = Arc::new(EmptyWaker);
    let waker = task::waker(empty);
    let mut cx = Context::from_waker(&waker);

    let actual_order = Rc::new(RefCell::new(Vec::new()));
    let mut st = Box::pin(f(actual_order.clone()));
    let _ = st.as_mut().poll_next(&mut cx);
    drop(st);

    let expected_order = Rc::new(RefCell::new(Vec::new()));
    g(expected_order.clone());
    assert_eq!(*actual_order.borrow(), *expected_order.borrow());
}

#[test]
fn test() {
    assert_drop_order_after_cancel(
        simple_variable_declaration_async,
        simple_variable_declaration_sync,
    );
    assert_drop_order_after_cancel(
        varable_completely_contained_within_block_async,
        varable_completely_contained_within_block_sync,
    );
    assert_drop_order_after_cancel(
        variables_moved_into_separate_blocks_async,
        variables_moved_into_separate_blocks_sync,
    );
    assert_drop_order_after_cancel(
        variables_moved_into_same_block_async,
        variables_moved_into_same_block_sync,
    );
    assert_drop_order_after_cancel(
        move_after_current_await_doesnt_affect_order,
        simple_variable_declaration_sync,
    );
}
