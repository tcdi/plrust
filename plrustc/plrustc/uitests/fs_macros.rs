#![crate_type = "lib"]
/*
Portions Copyright 2021-2025 Technology Concepts & Design, Inc.

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE file.
*/

const _A: &str = include_str!("fs_macros_included_file.txt");

const _B: &[u8] = include_bytes!("fs_macros_included_file.txt");

const _C: &str = core::include_str!("fs_macros_included_file.txt");
const _D: &[u8] = core::include_bytes!("fs_macros_included_file.txt");

macro_rules! indirect {
    ($callme:ident) => {
        $callme!("fs_macros_included_file.txt")
    };
}

const _E: &str = indirect!(include_str);
const _F: &[u8] = indirect!(include_bytes);

macro_rules! in_macro {
    () => {
        include_str!("fs_macros_included_file.txt")
    };
    ($invocation:expr) => {
        $invocation
    };
    ($mac:ident, $arg:expr) => {
        $mac!($arg)
    };
}

const _G: &str = in_macro!();
const _H: &str = in_macro!(include_str!("fs_macros_included_file.txt"));
const _I: &[u8] = in_macro!(include_bytes!("fs_macros_included_file.txt"));
const _J: &[u8] = in_macro!(include_bytes, "fs_macros_included_file.txt");

use core::include_str as sneaky;
const _L: &str = sneaky!("fs_macros_included_file.txt");
const _M: &str = in_macro!(sneaky, "fs_macros_included_file.txt");

fn _foo() -> String {
    format!("{:?}", in_macro!())
}
