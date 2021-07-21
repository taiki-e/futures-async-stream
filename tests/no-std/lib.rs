#![no_std]
#![warn(rust_2018_idioms, single_use_lifetimes)]
#![feature(generators)]

use futures_async_stream::{stream, try_stream};

include!("../include/basic.rs");
