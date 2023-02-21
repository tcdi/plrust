#![crate_type = "lib"]

fn _f() {
    let a = Box::new(1u32);
    let _b = Box::leak(a);

    let c = vec![1u8, 2, 3];
    let _d = c.leak();

    let _e = std::mem::forget(vec![1u32]);
}
