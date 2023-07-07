#![crate_type = "lib"]
use std::fmt;
trait Trait {
    type Associated;
}

impl<R, F: Fn() -> R> Trait for F {
    type Associated = R;
}

fn static_transfers_to_associated<T: Trait + 'static>(
    _: &T,
    x: T::Associated,
) -> Box<dyn fmt::Display /* + 'static */>
where
    T::Associated: fmt::Display,
{
    Box::new(x) // T::Associated: 'static follows from T: 'static
}

pub fn make_static_displayable<'a>(s: &'a str) -> Box<dyn fmt::Display> {
    let f = || -> &'a str { "" };
    // problem is: the closure type of `f` is 'static
    static_transfers_to_associated(&f, s)
}
