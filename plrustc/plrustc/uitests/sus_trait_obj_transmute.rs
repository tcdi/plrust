#![crate_type = "lib"]
/*
Portions Copyright 2021-2025 Technology Concepts & Design, Inc.

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE file.
*/

trait Object<U> {
    type Output;
}

impl<T: ?Sized, U> Object<U> for T {
    type Output = U;
}

fn foo<T: ?Sized, U>(x: <T as Object<U>>::Output) -> U {
    x
}

pub fn transmute<T, U>(x: T) -> U {
    foo::<dyn Object<U, Output = T>, U>(x)
}
