#![crate_type = "lib"]

pub struct Foo(pub std::cell::Cell<i32>, pub std::marker::PhantomPinned);

impl std::panic::UnwindSafe for Foo {}

impl std::panic::RefUnwindSafe for Foo {}

impl std::marker::Unpin for Foo {}
