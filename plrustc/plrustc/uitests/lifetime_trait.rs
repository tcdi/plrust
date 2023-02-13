#![crate_type = "lib"]
// #![forbid(plrust_lifetime_parameterized_traits)]
trait Foo<'a> {}

trait Bar {}

macro_rules! foobar {
    () => {
        trait Foobar<'a> {}
    };
}

foobar!();
