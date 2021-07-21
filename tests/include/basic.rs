#[stream(item = ())]
async fn stream() {}

#[stream(item = ())]
pub async fn for_await_in_stream_fn() {
    #[for_await]
    for () in stream() {
        yield;
        async {}.await;
    }
    yield;
    async {}.await;
}

#[try_stream(ok = (), error = ())]
pub async fn for_await_in_try_stream_fn() {
    #[for_await]
    for () in stream() {
        yield;
        async {}.await;
    }
    yield;
    async {}.await;
}

#[stream(item = ())]
pub async fn stream_in_stream_fn() {
    let _ = {
        #[stream]
        async move {
            yield;
            async {}.await;
        }
    };
    yield;
    async {}.await;
}

#[try_stream(ok = (), error = ())]
pub async fn stream_in_try_stream_fn() {
    let _ = {
        #[stream]
        async move {
            yield;
            async {}.await;
        }
    };
    yield;
    async {}.await;
}

#[stream(item = ())]
pub async fn try_stream_in_stream_fn() {
    let _ = {
        #[try_stream]
        async move {
            yield;
            async {}.await;
            // TODO: allow specifying error type in #[try_stream] attribute and remove this hack.
            return Err(());
        }
    };
    yield;
    async {}.await;
}

#[try_stream(ok = (), error = ())]
pub async fn try_stream_in_try_stream_fn() {
    let _ = {
        #[try_stream]
        async move {
            yield;
            async {}.await;
            // TODO: allow specifying error type in #[try_stream] attribute and remove this hack.
            return Err(());
        }
    };
    yield;
    async {}.await;
}
