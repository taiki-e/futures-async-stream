#![allow(incomplete_features)] // for impl_trait_in_bindings
#![feature(generators, impl_trait_in_bindings)]
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

// Check interoperability with rustc and clippy lints.

pub mod basic {
    use futures_async_stream::{stream, try_stream};
    use futures_core::stream::Stream;

    include!("include/basic.rs");
}

#[allow(box_pointers)]
#[allow(clippy::restriction)]
#[rustversion::attr(before(2020-11-26), ignore)] // Note: This date is commit-date and the day before the toolchain date.
#[test]
fn check_lint_list() {
    use std::{env, fs, path::Path, process::Command, str};

    type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

    fn assert_eq(expected_path: &str, actual: &str) -> Result<()> {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let expected_path = &manifest_dir.join(expected_path);
        let expected = fs::read_to_string(expected_path)?;
        if expected != actual {
            if env::var_os("CI").is_some() {
                let actual_path =
                    &manifest_dir.join("target").join(expected_path.file_name().unwrap());
                fs::write(actual_path, actual)?;
                let status = Command::new("git")
                    .args(&["--no-pager", "diff", "--no-index", "--"])
                    .args(&[expected_path, actual_path])
                    .status()?;
                assert!(!status.success());
                panic!("assertion failed");
            } else {
                fs::write(expected_path, actual)?;
            }
        }
        Ok(())
    }

    (|| -> Result<()> {
        let rustc = env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
        let output = Command::new(rustc).args(&["-W", "help"]).output()?;
        let new = str::from_utf8(&output.stdout)?;
        assert_eq("tests/lint.txt", new)
    })()
    .unwrap_or_else(|e| panic!("{}", e));
}
