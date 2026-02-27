#![crate_type = "lib"]
/*
Portions Copyright 2021-2025 Technology Concepts & Design, Inc.

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

pub fn foobar<'short, T>(r: &'short T) -> &'static T {
    fn foo<'out, 'input, T>(_dummy: &'out (), value: &'input T) -> (&'out &'input (), &'out T) {
        (&&(), value)
    }
    let foo1: for<'out, 'input> fn(&'out (), &'input T) -> (&'out &'input (), &'out T) = foo;
    let foo2: for<'input> fn(&'static (), &'input T) -> (&'static &'input (), &'static T) = foo1;
    let foo3: for<'input> fn(&'static (), &'input T) -> (&'input &'input (), &'static T) = foo2;
    let foo4: fn(&'static (), &'short T) -> (&'short &'short (), &'static T) = foo3;
    foo4(&(), r).1
}

pub fn should_be_allowed() {
    let a = &[1, 2, 3u8];
    // This should be allowed, as it's not a function pointer.
    a.iter().for_each(|v| {
        let _ignored = v;
    });
}
