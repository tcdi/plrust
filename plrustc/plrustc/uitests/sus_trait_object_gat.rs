#![crate_type = "lib"]
/*
Portions Copyright 2021-2025 Technology Concepts & Design, Inc.

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

trait Object {
    type Output;
}

impl<T: ?Sized> Object for T {
    type Output = &'static u64;
}

trait HasGenericAssoc {
    type Ohno<'a>: ?Sized;
}

impl HasGenericAssoc for () {
    type Ohno<'a> = dyn Object<Output = &'a u64>;
}

fn foo<'a, T: ?Sized>(x: <T as Object>::Output) -> &'a u64 {
    x
}

fn transmute_lifetime<'a, 'b>(x: &'a u64) -> &'b u64 {
    foo::<<() as HasGenericAssoc>::Ohno<'a>>(x)
}

pub fn get_dangling<'a>() -> &'a u64 {
    let x = 0;
    transmute_lifetime(&x)
}
