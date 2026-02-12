#![crate_type = "lib"]
/*
Portions Copyright 2021-2025 Technology Concepts & Design, Inc.

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE file.
*/
extern "C" {}
extern "Rust" {}
#[rustfmt::skip]
extern {}

macro_rules! foo {
    () => {
        extern "C" {}
    };
}

foo!();
