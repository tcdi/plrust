#![crate_type = "lib"]
/*
Portions Copyright 2021-2025 Technology Concepts & Design, Inc.

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

fn blah() {
    let _a: &dyn Fn() = &|| {};
    let _b: Box<dyn Fn()> = Box::new(|| {});
    let _c: Box<dyn FnMut()> = Box::new(|| {});
    let _d: Box<dyn FnOnce()> = Box::new(|| {});
}
