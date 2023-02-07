// HACK this crate 100% exists just to make it easier to smoke-test.
#![feature(plugin)]
#![plugin(plrust_plugins)]
// Real use would be `forbid`.
#![warn(extern_blocks, lifetime_parameterized_traits)]

extern "C" {}

macro_rules! not_in_macros_either {
    () => {
        extern "C" {}
    };
}
not_in_macros_either!();

pub trait Foo<'a> {}
