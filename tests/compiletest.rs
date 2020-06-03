#![warn(rust_2018_idioms, single_use_lifetimes)]

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
