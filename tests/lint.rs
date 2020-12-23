#![forbid(unsafe_code)]
#![warn(
    future_incompatible,
    nonstandard_style,
    rust_2018_compatibility,
    rust_2018_idioms,
    rustdoc,
    unused
)]
#![allow(unknown_lints)] // for old compilers
#![warn(
    box_pointers,
    deprecated_in_future,
    elided_lifetimes_in_paths,
    explicit_outlives_requirements,
    macro_use_extern_crate,
    meta_variable_misuse,
    missing_copy_implementations,
    missing_crate_level_docs,
    missing_debug_implementations,
    missing_docs,
    non_ascii_idents,
    single_use_lifetimes,
    trivial_casts,
    trivial_numeric_casts,
    unaligned_references,
    unreachable_pub,
    unused_extern_crates,
    unused_import_braces,
    unused_lifetimes,
    unused_qualifications,
    unused_results,
    variant_size_differences
)]
// absolute_paths_not_starting_with_crate, anonymous_parameters, keyword_idents, pointer_structural_match: warned as a part of future_incompatible
// missing_doc_code_examples, private_doc_tests, invalid_html_tags: warned as a part of rustdoc
// unsafe_block_in_unsafe_fn: unstable
// unsafe_code: forbidden
// unstable_features: deprecated: https://doc.rust-lang.org/beta/rustc/lints/listing/allowed-by-default.html#unstable-features
// unused_crate_dependencies: unrelated
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![warn(clippy::restriction)]
#![allow(clippy::blanket_clippy_restriction_lints)] // this is a test, so enable all restriction lints intentionally.
#![allow(clippy::implicit_return, clippy::let_underscore_must_use)]
#![allow(incomplete_features)] // for impl_trait_in_bindings
#![feature(generators, impl_trait_in_bindings)]

// Check interoperability with rustc and clippy lints.

mod auxiliary;

pub mod basic {
    use futures_async_stream::{stream, try_stream};
    use futures_core::stream::Stream;

    include!("include/basic.rs");
}

#[allow(clippy::restriction)]
#[rustversion::attr(before(2020-12-22), ignore)] // Note: This date is commit-date and the day before the toolchain date.
#[test]
fn check_lint_list() {
    use auxiliary::assert_diff;
    use std::{env, process::Command, str};

    let rustc = env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let output = Command::new(rustc).args(&["-W", "help"]).output().unwrap();
    let new = str::from_utf8(&output.stdout).unwrap();
    assert_diff("tests/lint.txt", new);
}
