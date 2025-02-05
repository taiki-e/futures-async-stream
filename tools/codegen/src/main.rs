// SPDX-License-Identifier: Apache-2.0 OR MIT

#![allow(clippy::needless_pass_by_value, clippy::wildcard_imports)]

#[macro_use]
mod file;

use crate::file::*;

fn main() {
    gen_assert_impl();
    gen_track_size();
}

fn gen_assert_impl() {
    let (path, out) = test_helper::codegen::gen_assert_impl(
        &workspace_root(),
        test_helper::codegen::AssertImplConfig {
            exclude: &[],
            not_send: &[],
            not_sync: &[],
            not_unpin: &[],
            not_unwind_safe: &[],
            not_ref_unwind_safe: &[],
        },
    );
    write(function_name!(), path, out).unwrap();
}

fn gen_track_size() {
    let (path, out) = test_helper::codegen::gen_track_size(
        &workspace_root(),
        test_helper::codegen::TrackSizeConfig { exclude: &[] },
    );
    write(function_name!(), path, out).unwrap();
}
