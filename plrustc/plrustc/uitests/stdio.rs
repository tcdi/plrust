#![crate_type = "lib"]
/*
Portions Copyright 2021-2025 Technology Concepts & Design, Inc.

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE file.
*/

fn _foo() {
    let _out = std::io::stdout();
    let _in = std::io::stdin();
    let _err = std::io::stderr();
}

fn _bar() {
    use std::io::*;
    let _out = stdout();
    let _in = stdin();
    let _err = stderr();
}

fn _baz() {
    use std::io::stdout as renamed;
    let _still_forbidden = renamed();
}

fn _quux() {
    let as_func = std::io::stdout;
    let _also_forbidden = as_func();
}
