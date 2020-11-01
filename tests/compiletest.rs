#![warn(rust_2018_idioms, single_use_lifetimes)]

use std::env;

#[rustversion::attr(before(2020-10-06), ignore)] // Note: This date is commit-date and the day before the toolchain date.
#[test]
fn ui() {
    if !env::var_os("CI").map_or(false, |v| v == "true") {
        env::set_var("TRYBUILD", "overwrite");
    }

    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
