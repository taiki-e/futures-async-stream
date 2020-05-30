#![feature(generators, proc_macro_hygiene, stmt_expr_attributes)]

use futures_async_stream::{for_await, stream};

#[stream(item = ())]
async fn def_site_arg1(a: Vec<u8>) {
    __arg0; //~ ERROR E0425
}

#[stream(item = ())]
async fn def_site_arg2(ref a: Vec<u8>) {
    __arg0; //~ ERROR E0425
}

#[stream(item = ())]
async fn def_site_task_context1() {
    __task_context; //~ ERROR E0425
}

#[stream(item = ())]
async fn def_site_task_context2() {
    async {}.await;
    __task_context; //~ ERROR E0425
}

#[stream(item = ())]
async fn stream() {}

#[stream(item = ())]
async fn def_site_for_await1() {
    #[for_await]
    for _ in stream() {
        &__pinned; //~ ERROR E0425
    }
}

async fn def_site_for_await2() {
    #[for_await]
    for _ in stream() {
        &__pinned; //~ ERROR E0425
    }
}

fn main() {}
