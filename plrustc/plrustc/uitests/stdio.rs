#![crate_type = "lib"]

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
