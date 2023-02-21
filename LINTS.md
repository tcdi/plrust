PL/Rust uses its own "rustc driver" so that it can employ custom lints to detect certain Rust code idioms and patterns
that trigger "I-Unsound" bugs in Rust itself.  Think "clippy" but built into the Rust compiler itself. In addition to 
these custom lints, PL/Rust uses some standard Rust lints to enforce safety.

In all cases, these lints are added to the generated code which wraps the user's `LANGUAGE plrust` function, as 
`#![forbid(${lint_name})]`.  They are used with "forbid" to ensure a user function cannot change it back to "allow".

PL/Rust does **not** apply these lints to dependant, external crates.  Dependencies *are* allowed to internally use 
whatever code they want, including `unsafe`.  Note that any public-facing `unsafe` functions won't be callable by a plrust 
function.

Dependencies are granted more freedom as the usable set can be controlled via the `plrust.allowed_dependencies` GUC.

**It is the administrator's responsibility to properly vet external dependencies for safety issues that may impact
the running environment.**

Any `LANGUAGE plrust` code that triggers any of the below lints will fail to compile, indicating the triggered lint.

# Standard Rust Lints

## `unknown_lints`

https://doc.rust-lang.org/rustc/lints/listing/warn-by-default.html#unknown-lints

PL/Rust won't allow any unknown (to our "rustc driver") lints to be applied.  The justification for this is to mainly
guard against type-os in the `plrust.compile_lints` GUC.

## `unsafe_code`

https://doc.rust-lang.org/rustc/lints/listing/allowed-by-default.html#unsafe-code

PL/Rust does not allow usage of `unsafe` code in `LANGUAGE plrust` functions.  This includes all the unsafe idioms such
as dereferencing pointers and calling other `unsafe` functions.

## `implied_bounds_entailment`

https://doc.rust-lang.org/rustc/lints/listing/warn-by-default.html#implied-bounds-entailment

This lint detects cases where the arguments of an impl method have stronger implied bounds than those from the trait 
method it's implementing.

If used incorrectly, this can be used to implement unsound APIs.

# PL/Rust `plrustc` Lints

## `plrust_extern_blocks`

This blocks the declaration of `extern "API" {}"` blocks.  Primarily, this is to ensure a plrust function cannot 
declare internal Postgres symbols as external.

For example, this code pattern is blocked:

```rust
extern "C" {
    pub fn palloc(size: Size) -> *mut ::std::os::raw::c_void;
}
```

## `plrust_lifetime_parameterized_traits`

Traits parameterized by lifetimes can be used to exploit Rust compiler bugs that lead to unsoundness issues.  PL/Rust
does not allow such traits to be declared.

For example, this code pattern is blocked:

```rust
    trait Foo<'a> {}
```

## `plrust_filesystem_macros`

Filesystem macros such as `include_bytes!` and `include_str!` are disallowed, as they provide access to the underlying filesystem which should be unavailable to a trusted language handler.

For example, this code pattern is blocked:

```rust
const SOMETHING: &str = include_str!("/etc/passwd");
```

## `plrust_fn_pointers`

Currently, several soundness holes have to do wuth the interaction between function pointers, implied bounds, and nested references. As a stopgap against these, use of function pointer types are currently blocked. This lint will likely be made more precise in the future.

Note that function types (such as the types resulting from closures as required by iterator functions) are still allowed, as these do not have the issues around variance.

For example, the following code pattern is blocked:

```rust
fn takes_fn_arg(x: fn()) {
    x();
}
```

## `plrust_async`

Currently async/await are forbidden by PL/Rust due to unclear interactions around lifetime and soundness constraints. This may be out of an overabundance of caution. Specifically, code like the following will fail to compile:

```rust
async fn an_async_fn() {
    // ...
}

fn normal_function() {
    let async_block = async {
        // ...
    };
    // ...
}
```

## `plrust_leaky`

This lint forbids use of "leaky" functions such as [`mem::forget`](https://doc.rust-lang.org/stable/std/mem/fn.forget.html) and [`Box::leak`](https://doc.rust-lang.org/stable/std/boxed/struct.Box.html#method.leak). While leaking memory is considered safe, it has undesirable effects and thus is blocked by default. For example, the lint will trigger on (at least) the following code:

```rust
core::mem::forget(something);
let foo = Box::leak(Box::new(1u32));
let bar = vec![1, 2, 3].leak();
```

Note that this will not prevent all leaks, as PL/Rust code could still create a leak by constructing a reference cycle using Rc/Arc, for example.
