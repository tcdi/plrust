#![crate_type = "lib"]

use core::ptr::NonNull;

trait Bad {
    fn bad(&mut self);
}
impl<T> Bad for Box<T> {
    fn bad(&mut self) {
        let Self(ptr, _) = self;

        fn dangling<T, U>() -> U
        where
            U: From<NonNull<T>>,
        {
            NonNull::dangling().into()
        }

        *ptr = dangling();
    }
}

fn main() {
    let mut foo = Box::new(123);
    foo.bad();
}
