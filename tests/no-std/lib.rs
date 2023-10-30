// SPDX-License-Identifier: Apache-2.0 OR MIT

#![no_std]
#![feature(coroutines)]
#![allow(clippy::no_effect_underscore_binding)] // broken

use futures_async_stream::{stream, try_stream};

include!("../include/basic.rs");
