#![crate_type = "lib"]

// Should be allowed.
pub mod blah {}

// Should not be allowed.
#[path = "external_mod_included_file.txt"]
pub mod foo;

// Also should not be allowed, but I'm not really sure how to test it...

// pub mod also_disallowed;
