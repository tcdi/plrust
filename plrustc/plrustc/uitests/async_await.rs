#![crate_type = "lib"]
/*
Portions Copyright 2021-2025 Technology Concepts & Design, Inc.

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

async fn foo() {}

fn bar() {
    let _ = async { 1 };
}

async fn baz() -> i32 {
    async { 1 }.await
}
