// SPDX-License-Identifier: Apache-2.0 OR MIT

#![no_std]
#![feature(coroutines)]

use futures_async_stream::{stream, try_stream};

include!("../include/basic.rs");
