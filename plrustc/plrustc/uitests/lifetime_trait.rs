#![crate_type = "lib"]
/*
Portions Copyright 2021-2025 Technology Concepts & Design, Inc.

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/
// #![forbid(plrust_lifetime_parameterized_traits)]
trait Foo<'a> {}

trait Bar {}

macro_rules! foobar {
    () => {
        trait Foobar<'a> {}
    };
}

foobar!();
