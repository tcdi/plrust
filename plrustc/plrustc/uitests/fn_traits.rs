#![crate_type = "lib"]

fn blah() {
    let _a: &dyn Fn() = &|| {};
    let _b: Box<dyn Fn()> = Box::new(|| {});
    let _c: Box<dyn FnMut()> = Box::new(|| {});
    let _d: Box<dyn FnOnce()> = Box::new(|| {});
}
