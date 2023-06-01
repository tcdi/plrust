#![crate_type = "lib"]

mod my {
    pub struct Foo(&'static str);
}
impl AsRef<str> for my::Foo {
    fn as_ref(&self) -> &str {
        let Self(s) = self;
        s
    }
}
