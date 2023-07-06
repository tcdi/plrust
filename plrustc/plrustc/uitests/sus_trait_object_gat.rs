#![crate_type = "lib"]

trait Object {
    type Output;
}

impl<T: ?Sized> Object for T {
    type Output = &'static u64;
}

trait HasGenericAssoc {
    type Ohno<'a>: ?Sized;
}

impl HasGenericAssoc for () {
    type Ohno<'a> = dyn Object<Output = &'a u64>;
}

fn foo<'a, T: ?Sized>(x: <T as Object>::Output) -> &'a u64 {
    x
}

fn transmute_lifetime<'a, 'b>(x: &'a u64) -> &'b u64 {
    foo::<<() as HasGenericAssoc>::Ohno<'a>>(x)
}

pub fn get_dangling<'a>() -> &'a u64 {
    let x = 0;
    transmute_lifetime(&x)
}
