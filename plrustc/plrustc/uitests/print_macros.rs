#![crate_type = "lib"]
/*
Portions Copyright 2021-2025 Technology Concepts & Design, Inc.

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE file.
*/

fn _foo() {
    print!("hello");
    println!("world");

    eprint!("test");
    eprintln!("123");

    dbg!("baz");
}

fn _bar() {
    use std::{dbg as d, eprint as e, eprintln as eln, print as p, println as pln};
    p!("hello");
    pln!("world");

    e!("test");
    eln!("123");

    d!("baz");
}

macro_rules! wrapped {
    () => {{
        print!("hello");
        println!("world");

        eprint!("test");
        eprintln!("123");

        dbg!("baz");
    }};
}

fn _baz() {
    wrapped!();
}

macro_rules! indirect {
    ($invocation:expr) => {
        $invocation
    };
    ($mac:ident, $arg:expr) => {
        $mac!($arg)
    };
    (@call $mac:ident) => {
        $mac!()
    };
}

fn _indir() {
    indirect!(println!("foo"));
    indirect!(println, "foo");
}

fn _indir2() {
    indirect!(wrapped!());
    indirect!(@call wrapped);
}
