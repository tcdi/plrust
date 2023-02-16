#![crate_type = "lib"]

pub fn foobar<'short, T>(r: &'short T) -> &'static T {
    fn foo<'out, 'input, T>(_dummy: &'out (), value: &'input T) -> (&'out &'input (), &'out T) {
        (&&(), value)
    }
    let foo1: for<'out, 'input> fn(&'out (), &'input T) -> (&'out &'input (), &'out T) = foo;
    let foo2: for<'input> fn(&'static (), &'input T) -> (&'static &'input (), &'static T) = foo1;
    let foo3: for<'input> fn(&'static (), &'input T) -> (&'input &'input (), &'static T) = foo2;
    let foo4: fn(&'static (), &'short T) -> (&'short &'short (), &'static T) = foo3;
    foo4(&(), r).1
}
