#![feature(generators, proc_macro_hygiene, stmt_expr_attributes)]

mod for_await {
    use futures_async_stream::{for_await, stream};

    #[stream(item = ())]
    async fn stream() {}

    async fn unexpected1() {
        #[for_await(bar)] //~ ERROR unexpected token
        for () in stream() {}
    }

    async fn unexpected2() {
        #[for_await()] // Ok
        for () in stream() {}
    }

    #[stream(item = i32)]
    async fn unexpected_in_fn1() {
        #[for_await(bar)] //~ ERROR unexpected token
        for () in stream() {}
    }

    #[stream(item = i32)]
    async fn unexpected_in_fn2() {
        #[for_await()] //~ ERROR unexpected token
        for () in stream() {}
    }
}

mod stream {
    use futures_async_stream::stream;

    #[stream] //~ ERROR unexpected end of input, expected `item`
    async fn expected_item() {}

    #[stream(item)] //~ ERROR expected `=`
    async fn expected_eq() {}

    #[stream(item = )] //~ ERROR unexpected end of input, expected one of
    async fn expected_ty() {}

    #[stream(baz, item = i32)] //~ ERROR expected `item`
    async fn unexpected_first() {}

    #[stream(item = i32, baz)] //~ ERROR unexpected argument
    async fn unexpected_second() {}

    #[stream(boxed, item = i32)] // Ok
    async fn boxed_first() {}

    #[stream(,item = i32)] //~ ERROR expected `item`
    async fn unexpected_comma() {}

    #[stream(item = i32 item = i32)] //~ ERROR expected `,`
    async fn expected_comma() {}

    #[stream(item = i32, item = i32)] //~ ERROR duplicate `item` argument
    async fn duplicate_item() {}

    #[stream(item = i32, boxed, boxed)] //~ ERROR duplicate `boxed` argument
    async fn duplicate_boxed() {}

    #[stream(item = i32, boxed_local, boxed_local)] //~ ERROR duplicate `boxed_local` argument
    async fn duplicate_boxed_local() {}

    #[stream(item = i32, boxed_local, boxed)] //~ ERROR `boxed` and `boxed_local` cannot be used at the same time.
    async fn combine() {}
}

mod try_stream {
    use futures_async_stream::try_stream;

    #[try_stream] //~ ERROR unexpected end of input, expected `ok`
    async fn expected_ok1() {}

    #[try_stream(error = ())] //~ ERROR unexpected end of input, expected `ok`
    async fn expected_ok2() {}

    #[try_stream(ok)] //~ ERROR expected `=`
    async fn expected_ok_eq() {}

    #[try_stream(ok = )] //~ ERROR unexpected end of input, expected one of
    async fn expected_ok_ty() {}

    #[try_stream(ok = ())] //~ ERROR unexpected end of input, expected `error`
    async fn expected_error() {}

    #[try_stream(error)] //~ ERROR expected `=`
    async fn expected_error_eq() {}

    #[try_stream(error = )] //~ ERROR unexpected end of input, expected one of
    async fn expected_error_ty() {}

    #[try_stream(baz, ok = (), error = ())] //~ ERROR expected `ok`
    async fn unexpected_first() {}

    #[try_stream(ok = (), baz, error = ())] //~ ERROR expected `error`
    async fn unexpected_second() {}

    #[try_stream(ok = (), error = (), baz)] //~ ERROR unexpected argument
    async fn unexpected_third() {}

    #[try_stream(boxed, ok = (), error = ())] // Ok
    async fn boxed_first() {}

    #[try_stream(,ok = () error = ())] //~ ERROR expected `ok`
    async fn unexpected_comma() {}

    #[try_stream(ok = () error = ())] //~ ERROR expected `,`
    async fn expected_comma1() {}

    #[try_stream(ok = (), error = () error = ())] //~ ERROR expected `,`
    async fn expected_comma2() {}

    #[try_stream(ok = (), ok = (), error = ())] //~ ERROR duplicate `ok` argument
    async fn duplicate_ok1() {}

    #[try_stream(ok = (), error = (), ok = (), error = ())] //~ ERROR duplicate `ok` argument
    async fn duplicate_ok2() {}

    #[try_stream(ok = (), error = (), error = ())] //~ ERROR duplicate `error` argument
    async fn duplicate_error() {}

    #[try_stream(ok = (), error = (), boxed, boxed)] //~ ERROR duplicate `boxed` argument
    async fn duplicate_boxed() {}

    #[try_stream(ok = (), error = (), boxed_local, boxed_local)] //~ ERROR duplicate `boxed_local` argument
    async fn duplicate_boxed_local() {}

    #[try_stream(ok = (), error = (), boxed_local, boxed)] //~ ERROR `boxed` and `boxed_local` cannot be used at the same time.
    async fn combine() {}
}

fn main() {}
