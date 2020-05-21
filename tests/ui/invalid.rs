#![feature(generators, proc_macro_hygiene, stmt_expr_attributes)]

mod signature {
    use futures_async_stream::stream;

    #[stream(item = ())]
    const fn constness() {} //~ ERROR async stream must be declared as async

    #[stream(item = ())]
    fn variadic(_: ...) {} //~ ERROR only foreign functions are allowed to be C-variadic

    #[stream(item = ())]
    fn asyncness() {} //~ ERROR async stream must be declared as async

    #[stream(item = ())]
    async fn output() -> i32 {} //~ ERROR async stream must return the unit type

    #[stream(item = ())]
    async fn unit() -> () {} // OK

    #[stream(item = ())]
    const async unsafe extern "C" fn f() {} //~ ERROR async stream may not be const
}

mod attribute {
    use futures_async_stream::{for_await, stream, try_stream};

    #[stream(item = ())]
    async fn stream() {}

    #[stream(item = ())]
    #[stream(item = ())] //~ ERROR duplicate #[stream] attribute
    async fn duplicate_stream_fn() {}

    #[try_stream(ok = (), error = ())]
    #[try_stream(ok = (), error = ())] //~ ERROR duplicate #[try_stream] attribute
    async fn duplicate_try_stream_fn() {}

    #[stream(item = ())]
    #[try_stream(ok = (), error = ())] //~ ERROR may not be used at the same time
    async fn combine_fn() {}

    // span is lost.
    // Refs: https://github.com/rust-lang/rust/issues/43081
    //~ ERROR duplicate #[stream] attribute
    async fn duplicate_stream_async() {
        let _ = {
            #[stream]
            #[stream]
            async move {}
        };
    }

    // span is lost.
    // Refs: https://github.com/rust-lang/rust/issues/43081
    //~ ERROR duplicate #[try_stream] attribute
    async fn duplicate_try_stream_async() {
        let _ = {
            #[try_stream]
            #[try_stream]
            async move {}
        };
    }

    async fn duplicate_for_await() {
        #[for_await] //~ ERROR duplicate #[for_await] attribute
        #[for_await]
        for () in stream() {}
    }

    // span is lost.
    // Refs: https://github.com/rust-lang/rust/issues/43081
    //~ ERROR may not be used at the same time
    async fn combine_async() {
        let _ = {
            #[stream]
            #[try_stream]
            async move {}
        };
    }

    #[stream(item = ())]
    async fn duplicate_stream_async_in_fn() {
        let _ = {
            #[stream]
            #[stream] //~ ERROR duplicate #[stream] attribute
            async move {}
        };
    }

    #[stream(item = ())]
    async fn duplicate_try_stream_async_in_fn() {
        let _ = {
            #[try_stream]
            #[try_stream] //~ ERROR duplicate #[try_stream] attribute
            async move {}
        };
    }

    #[stream(item = ())]
    async fn duplicate_for_await_in_fn() {
        #[for_await]
        #[for_await] //~ ERROR duplicate #[for_await] attribute
        for () in stream() {}
    }

    #[stream(item = ())]
    async fn combine_async_in_fn() {
        let _ = {
            #[stream]
            #[try_stream] //~ ERROR may not be used at the same time
            async move {}
        };
    }
}

mod item {
    use futures_async_stream::stream;

    #[stream(item = ())] //~ ERROR #[stream] attribute may only be used on async functions or async blocks
    mod m {}

    #[stream(item = ())] //~ ERROR #[stream] attribute may only be used on async functions or async blocks
    trait A {}

    #[stream(item = ())] //~ ERROR #[stream] attribute may only be used on async functions or async blocks
    impl A {}
}

fn main() {}
