// SPDX-License-Identifier: Apache-2.0 OR MIT

#![allow(clippy::needless_pass_by_value, clippy::wildcard_imports)]

use std::path::Path;

use test_helper::{bin_name, codegen::file, function_name};

fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR").strip_suffix("tools/codegen").unwrap())
}

fn main() {
    gen_assert_impl();
    gen_track_size();
}

fn gen_assert_impl() {
    let workspace_root = workspace_root();
    let (path, out) = test_helper::codegen::gen_assert_impl(
        workspace_root,
        test_helper::codegen::AssertImplConfig {
            exclude: &[],
            not_send: &[],
            not_sync: &[],
            not_unpin: &[],
            not_unwind_safe: &[],
            not_ref_unwind_safe: &[],
        },
    );
    file::write(function_name!(), bin_name!(), workspace_root, path, out);
}

fn gen_track_size() {
    let workspace_root = workspace_root();
    let (path, out) = test_helper::codegen::gen_track_size(
        workspace_root,
        test_helper::codegen::TrackSizeConfig { exclude: &[] },
    );
    file::write(function_name!(), bin_name!(), workspace_root, path, out);
}
