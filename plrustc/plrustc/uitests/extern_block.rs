#![crate_type = "lib"]
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
