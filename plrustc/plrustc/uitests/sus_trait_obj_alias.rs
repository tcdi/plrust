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

fn foo<'a, T: ?Sized>(x: <T as Object>::Output) -> &'a u64 {
    x
}

fn transmute_lifetime<'a, 'b>(x: &'a u64) -> &'b u64 {
    type A<'x> = dyn Object<Output = &'x u64>;
    type B<'x> = A<'x>;
    foo::<B<'a>>(x)
}

// And yes this is a genuine `transmute_lifetime`!
fn get_dangling<'a>() -> &'a u64 {
    let x = 0;
    transmute_lifetime(&x)
}

pub fn problems() -> &'static u64 {
    get_dangling()
}
