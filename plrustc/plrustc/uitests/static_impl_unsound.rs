#![crate_type = "lib"]

use std::fmt::Display;

trait Displayable {
    fn display(self) -> Box<dyn Display>;
}

// This is more complex than the one in the issue, to make sure the `Box`'s
// lang_item status doesn't bite us.
impl<T: Display> Displayable for (T, Box<Option<&'static T>>) {
    fn display(self) -> Box<dyn Display> {
        Box::new(self.0)
    }
}

fn extend_lt<T, U>(val: T) -> Box<dyn Display>
where
    (T, Box<Option<U>>): Displayable,
{
    Displayable::display((val, Box::new(None)))
}

pub fn get_garbage(s: &str) -> String {
    let val = extend_lt(&String::from(s));
    val.to_string()
}
