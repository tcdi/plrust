/*
Portions Copyright 2021-2025 Technology Concepts & Design, Inc.

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/
// rustc-env:RUST_BACKTRACE=0
// rustc-env:PLRUSTC_INCLUDE_TEST_ONLY_LINTS=1
// normalize-stderr-test: "plrustc version: .*" -> "plrustc version: <version here>"
// normalize-stderr-test: "force_ice.rs:\d*:\d*" -> "force_ice.rs"
// normalize-stderr-test: "(?ms)query stack during panic:\n.*end of query stack\n" -> ""
// normalize-stderr-test: "note: rustc .*? running on .*" -> "note: rustc <version here> running on <target here>"
#![crate_type = "lib"]
// The comments above are to clean up file/line/version numbers, backtrace info,
// etc. We want to avoid ice_hook.stderr changing more than is needed.

// This function name is special-cased in `PlrustcForceIce`
pub fn plrustc_would_like_some_ice() {}
