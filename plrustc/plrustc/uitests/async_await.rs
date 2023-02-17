#![crate_type = "lib"]

async fn foo() {}

fn bar() {
    let _ = async { 1 };
}

async fn baz() -> i32 {
    async { 1 }.await
}
