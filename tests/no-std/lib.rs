#![no_std]
#![warn(rust_2018_idioms, single_use_lifetimes)]
#![allow(incomplete_features)] // for impl_trait_in_bindings
#![feature(generators, impl_trait_in_bindings)]

use futures_async_stream::{stream, try_stream};
use futures_core::stream::Stream;

include!("../include/basic.rs");
