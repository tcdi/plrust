#![crate_type = "lib"]

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
