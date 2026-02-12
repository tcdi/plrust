#![crate_type = "lib"]
/*
Portions Copyright 2021-2025 Technology Concepts & Design, Inc.

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE file.
*/

use core::ptr::NonNull;

trait Bad {
    fn bad(&mut self);
}
impl<T> Bad for Box<T> {
    fn bad(&mut self) {
        let Self(ptr, _) = self;

        fn dangling<T, U>() -> U
        where
            U: From<NonNull<T>>,
        {
            NonNull::dangling().into()
        }

        *ptr = dangling();
    }
}

fn main() {
    let mut foo = Box::new(123);
    foo.bad();
}
