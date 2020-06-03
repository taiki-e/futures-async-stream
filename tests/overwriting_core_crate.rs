#![warn(rust_2018_idioms, single_use_lifetimes)]
#![allow(incomplete_features)]
#![feature(generators, impl_trait_in_bindings)]

// See https://github.com/rust-lang/pin-utils/pull/26#discussion_r344491597
//
// Note: If the proc-macro does not depend on its own items, it may be preferable not to
//       support overwriting the name of core/std crate for compatibility with reexport.
#[allow(unused_extern_crates)]
extern crate futures_async_stream as core;

// Dummy module to check that the expansion refers to the crate.
mod futures_async_stream {}

use ::futures_async_stream::{stream, try_stream};
use futures_core::stream::Stream;

include!("include/basic.rs");
