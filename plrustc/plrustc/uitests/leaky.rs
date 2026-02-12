#![crate_type = "lib"]
/*
Portions Copyright 2021-2025 Technology Concepts & Design, Inc.

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE file.
*/

fn _f() {
    let a = Box::new(1u32);
    let _b = Box::leak(a);

    let c = vec![1u8, 2, 3];
    let _d = c.leak();

    let e = vec![1u8, 2, 3];
    let _f = Vec::leak(e);

    let _g = std::mem::forget(vec![1u32]);

    let vec_leak = Vec::<u8>::leak;
    let _ = vec_leak(vec![1, 2, 3u8]);
}
