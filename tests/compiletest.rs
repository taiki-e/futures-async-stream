#![cfg(compiletest)]
#![warn(rust_2018_idioms)]

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
