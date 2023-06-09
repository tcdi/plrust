#![crate_type = "lib"]

pub struct Foo(&'static str);

impl AsRef<str> for Foo {
    fn as_ref(&self) -> &str {
        let Self(s) = self;
        s
    }
}
