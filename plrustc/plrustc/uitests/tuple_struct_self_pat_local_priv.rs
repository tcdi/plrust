#![crate_type = "lib"]
/*
Portions Copyright 2021-2025 Technology Concepts & Design, Inc.

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE file.
*/

mod my {
    pub struct Foo(&'static str);
}
impl AsRef<str> for my::Foo {
    fn as_ref(&self) -> &str {
        let Self(s) = self;
        s
    }
}
