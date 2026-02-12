#![crate_type = "lib"]
/*
Portions Copyright 2021-2025 Technology Concepts & Design, Inc.

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE file.
*/

trait Foo {}
struct Bar<A, B>(core::marker::PhantomData<(A, B)>);
struct Baz<'a, A>(core::marker::PhantomData<&'a A>);
trait Quux<A> {}

impl<T> Foo for &'static T {}
impl<T> Foo for &'static mut T {}
impl<T> Foo for [&'static T] {}
impl<T> Foo for &[&'static T] {}
impl<T> Foo for (i32, [&'static T]) {}
impl<T> Foo for (i32, [&'static T; 1]) {}
impl<T> Foo for *const &'static T {}
impl<T> Foo for Bar<i32, &'static T> {}
impl<T> Foo for Baz<'static, T> {}
impl<T> Foo for dyn Quux<&'static T> {}
impl<T> Foo for &'static dyn Quux<T> {}
