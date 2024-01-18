// SPDX-License-Identifier: Apache-2.0 OR MIT

#![feature(coroutines)]

use futures_async_stream::stream;

#[stream(item = L)] //~ ERROR cannot find type `L` in this scope
async fn foo() {}

fn main() {}
