#![crate_type = "lib"]
/*
Portions Copyright 2021-2025 Technology Concepts & Design, Inc.

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

// Should be allowed.
pub mod blah {}

// Should not be allowed.
#[path = "external_mod_included_file.txt"]
pub mod foo;

// Also should not be allowed, but I'm not really sure how to test it...

// pub mod also_disallowed;
