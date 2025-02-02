// SPDX-License-Identifier: Apache-2.0 OR MIT

#![allow(clippy::wildcard_imports)]

use core::marker::PhantomPinned;

use static_assertions::{assert_impl_all as assert_impl, assert_not_impl_all as assert_not_impl};

use crate::*;

// TODO: generate this with codegen
assert_impl!(future::GenFuture<()>: Send);
assert_not_impl!(future::GenFuture<*const ()>: Send);
assert_impl!(future::GenFuture<()>: Sync);
assert_not_impl!(future::GenFuture<*const ()>: Sync);
assert_impl!(future::GenFuture<()>: Unpin);
assert_not_impl!(future::GenFuture<PhantomPinned>: Unpin);

assert_impl!(stream::GenStream<()>: Send);
assert_not_impl!(stream::GenStream<*const ()>: Send);
assert_impl!(stream::GenStream<()>: Sync);
assert_not_impl!(stream::GenStream<*const ()>: Sync);
assert_impl!(stream::GenStream<()>: Unpin);
assert_not_impl!(stream::GenStream<PhantomPinned>: Unpin);

assert_impl!(stream::Next<'_, ()>: Send);
assert_not_impl!(stream::Next<'_, *const ()>: Send);
assert_impl!(stream::Next<'_, ()>: Sync);
assert_not_impl!(stream::Next<'_, *const ()>: Sync);
assert_impl!(stream::Next<'_, PhantomPinned>: Unpin);

assert_impl!(try_stream::GenTryStream<()>: Send);
assert_not_impl!(try_stream::GenTryStream<*const ()>: Send);
assert_impl!(try_stream::GenTryStream<()>: Sync);
assert_not_impl!(try_stream::GenTryStream<*const ()>: Sync);
assert_impl!(try_stream::GenTryStream<()>: Unpin);
assert_not_impl!(try_stream::GenTryStream<PhantomPinned>: Unpin);
