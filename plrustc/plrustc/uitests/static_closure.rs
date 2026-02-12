#![crate_type = "lib"]
/*
Portions Copyright 2021-2025 Technology Concepts & Design, Inc.

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE file.
*/
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
