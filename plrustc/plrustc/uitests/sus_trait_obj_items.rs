#![crate_type = "lib"]
/*
Portions Copyright 2021-2025 Technology Concepts & Design, Inc.

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE file.
*/

pub trait Foo {}

pub trait Bar<T = dyn Foo>
where
    T: ?Sized,
{
}

#[allow(invalid_type_param_default)] // not the lint we're interested in testing
pub fn sus_fn<T = dyn Foo>()
where
    T: ?Sized,
{
}

pub struct SusStruct<T = dyn Foo>(pub Box<T>)
where
    T: ?Sized;

pub enum SusEnum<T = dyn Foo>
where
    T: ?Sized,
{
    Something(Box<T>),
}

pub union SusUnion<T = dyn Foo>
where
    T: ?Sized,
{
    pub something: *const T,
}
