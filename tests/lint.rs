#![warn(rust_2018_idioms, single_use_lifetimes)]
#![allow(incomplete_features)]
#![feature(generators, impl_trait_in_bindings)]
#![warn(unused, future_incompatible)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

pub mod basic {
    #![forbid(unsafe_code)]

    use futures_async_stream::{stream, try_stream};
    use futures_core::stream::Stream;

    include!("include/basic.rs");
}
