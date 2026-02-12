#![crate_type = "lib"]
/*
Portions Copyright 2021-2025 Technology Concepts & Design, Inc.

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE file.
*/

pub struct Foo(pub std::cell::Cell<i32>, pub std::marker::PhantomPinned);

impl std::panic::UnwindSafe for Foo {}

impl std::panic::RefUnwindSafe for Foo {}

impl std::marker::Unpin for Foo {}
