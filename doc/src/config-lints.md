# Lints


PL/Rust has its own "rustc driver" named `plrustc`.  This must be installed using the 
[`plrustc/build.sh`](plrustc/build.sh) script and the resulting executable must be on the `PATH`, or it should reside
somewhere that is included in the `plrust.PATH_override` GUC.


PL/Rust uses its own "rustc driver" so that it can employ custom lints to detect certain Rust code idioms and patterns
that trigger "I-Unsound" bugs in Rust itself.  Think "clippy" but built into the Rust compiler itself. In addition to 
these custom lints, PL/Rust uses some standard Rust lints to enforce safety.


The `plrust.required_lints` GUC defines which lints must have been applied to a function before PL/Rust will load the
library and execute the function.  Using the `PLRUST_REQUIRED_LINTS` environment variable, it is possible to enforce
that certain lints are always required of compiled functions, regardless of the `plrust.required_lints` GUC value.
`PLRUST_REQUIRED_LINTS`'s format is a comma-separated list of lint named.  It must be set in the environment in which 
Postgres is started.  The intention here is that the system administrator can force certain lints for execution if for 
some reason `postgresql.conf` or the users able to modify it are not trusted.


In all cases, these lints are added to the generated code which wraps the user's `LANGUAGE plrust` function, as 
`#![forbid(${lint_name})]`.  They are used with "forbid" to ensure a user function cannot change it back to "allow".

PL/Rust does **not** apply these lints to dependant, external crates.  Dependencies *are* allowed to internally use 
whatever code they want, including `unsafe`.  Note that any public-facing `unsafe` functions won't be callable by a plrust 
function.

Dependencies are granted more freedom as the usable set can be controlled via the `plrust.allowed_dependencies` GUC.

----

**It is the administrator's responsibility to properly vet external dependencies for safety issues that may impact
the running environment.**

----

Any `LANGUAGE plrust` code that triggers any of the below lints will fail to compile, indicating the triggered lint.

## Standard Rust Lints

### `unknown_lints`

https://doc.rust-lang.org/rustc/lints/listing/warn-by-default.html#unknown-lints

PL/Rust won't allow any unknown (to our "rustc driver") lints to be applied.  The justification for this is to mainly
guard against type-os in the `plrust.compile_lints` GUC.

### `unsafe_code`

https://doc.rust-lang.org/rustc/lints/listing/allowed-by-default.html#unsafe-code

PL/Rust does not allow usage of `unsafe` code in `LANGUAGE plrust` functions.  This includes all the unsafe idioms such
as dereferencing pointers and calling other `unsafe` functions.

### `implied_bounds_entailment`

https://doc.rust-lang.org/rustc/lints/listing/warn-by-default.html#implied-bounds-entailment

This lint detects cases where the arguments of an impl method have stronger implied bounds than those from the trait 
method it's implementing.

If used incorrectly, this can be used to implement unsound APIs.

### `deprecated`

https://doc.rust-lang.org/rustc/lints/listing/warn-by-default.html#deprecated

The deprecated lint detects use of deprecated items. This is forbidden because certain items in the Rust standard library are incorrectly-safe APIs but were only deprecated rather than removed when a version with the appropriate safety annotation was added.

### `suspicious_auto_trait_impls`

https://doc.rust-lang.org/rustc/lints/listing/warn-by-default.html#suspicious-auto-trait-impls

This defends against some patterns that can lead to soundness issues. These cases currently can only trigger in patterns which are otherwise blocked by the `unsafe_code` lint, but for better defense-in-depth, it's explicitly forbidden in PL/Rust.

### `unaligned_references`

https://doc.rust-lang.org/rustc/lints/listing/deny-by-default.html#unaligned-references

The unaligned_references lint detects unaligned references to fields of packed structs. This forbidden because it is a soundness hole in the language.

### `soft_unstable`

https://doc.rust-lang.org/rustc/lints/listing/deny-by-default.html#soft-unstable

This prevents the use of language and library features which were accidentally stabilized. This is forbidden because there's no reason to need to use these, and forbidding them reduces the set of APIs and features we have to consider in PL/Rust.

### `where_clauses_object_safety`

https://doc.rust-lang.org/rustc/lints/listing/warn-by-default.html#where-clauses-object-safety

This avoids some soundness holes that are in the language which can be used to trigger various crashes, see the lint documentation for details.

## PL/Rust `plrustc` Lints

### `plrust_extern_blocks`

This blocks the declaration of `extern "API" {}"` blocks.  Primarily, this is to ensure a plrust function cannot 
declare internal Postgres symbols as external.

For example, this code pattern is blocked:

```rust
extern "C" {
    pub fn palloc(size: Size) -> *mut ::std::os::raw::c_void;
}
```

### `plrust_lifetime_parameterized_traits`

Traits parameterized by lifetimes can be used to exploit Rust compiler bugs that lead to unsoundness issues.  PL/Rust
does not allow such traits to be declared.

For example, this code pattern is blocked:

```rust
    trait Foo<'a> {}
```

### `plrust_filesystem_macros`

Filesystem macros such as `include_bytes!` and `include_str!` are disallowed, as they provide access to the underlying filesystem which should be unavailable to a trusted language handler.

For example, this code pattern is blocked:

```rust
const SOMETHING: &str = include_str!("/etc/passwd");
```

### `plrust_fn_pointers`

Currently, several soundness holes have to do with the interaction between function pointers, implied bounds, and nested references. As a stopgap against these, use of function pointer types and function trait objects are currently blocked. This lint will likely be made more precise in the future.

Note that function types (such as the types resulting from closures as required by iterator functions) are still allowed, as these do not have the issues around variance.

For example, the following code pattern is blocked:

```rust
fn takes_fn_arg(x: fn()) {
    x();
}
```

### `plrust_async`

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

### `plrust_leaky`

This lint forbids use of "leaky" functions such as [`mem::forget`](https://doc.rust-lang.org/stable/std/mem/fn.forget.html) and [`Box::leak`](https://doc.rust-lang.org/stable/std/boxed/struct.Box.html#method.leak). While leaking memory is considered safe, it has undesirable effects and thus is blocked by default. For example, the lint will trigger on (at least) the following code:

```rust
core::mem::forget(something);
let foo = Box::leak(Box::new(1u32));
let bar = vec![1, 2, 3].leak();
```

Note that this will not prevent all leaks, as PL/Rust code could still create a leak by constructing a reference cycle using Rc/Arc, for example.

### `plrust_env_macros`

This lint forbids use of environment macros such as [`env!`](https://doc.rust-lang.org/nightly/std/macro.env.html) and [`option_env!`](https://doc.rust-lang.org/nightly/std/macro.option_env.html), as it allows access to data that should not be available to a trusted language handler.

```rust
let path = env!("PATH");
let rustup_toolchain_dir = option_env!("RUSTUP_TOOLCHAIN");
// ...
```

### `plrust_external_mod`

This lint forbids use of non-inline `mod blah`, as it can be used to access files a trusted language handler should not give access to.

```rust
// This is allowed
mod foo {
    // some functions or whatever here...
}

// This is disallowed.
mod bar;
// More importantly, this is disallowed as well.
#[path = "/sneaky/path/to/something"]
mod baz;
```


### `plrust_print_macros`

This lint forbids use of the `println!`/`eprintln!` family of macros (including `dbg!` and the non-`ln` variants), as these allow bypassing the norm. Users should use `pgrx::log!` or `pgrx::debug!` instead.

```rust
println!("hello");
print!("plrust");

eprintln!("this is also blocked");
eprint!("even without the newline");

dbg!("same here");
```

### `plrust_stdio`

This lint forbids use of the functions for accessing standard streams (stdin, stdout, stderr) from PL/Rust, for the same reason as above. For example, the following code is forbidden:

```rust
std::io::stdout().write_all(b"foobar").unwrap();
std::io::stderr().write_all(b"foobar").unwrap();
let _stdin_is_forbidden_too = std::io::stdin();
```

### `plrust_static_impls`

This lint forbids certain `impl` blocks for types containing `&'static` references. The precise details are somewhat obscure, but can usually be avoided by making a custom struct to contain your static reference, which avoids the particular soundness hole we're concerned with. For example:

```rust
// This is forbidden:
impl SomeTrait for (&'static Foo, Bar) {
    // ...
}

// Instead, do this:
struct MyType(&'static Foo, Bar);
impl SomeTrait for MyType {
    // ...
}
```

### `plrust_autotrait_impls`

This lint forbids explicit implementations of the safe auto traits, as a workaround for various soundness holes around these. It may be relaxed in the future if those are fixed.

```rust
struct Foo(std::cell::Cell<i32>, std::marker::PhantomPinned);
// Any of the following implementations would be forbidden.
impl std::panic::UnwindSafe for Foo {}
impl std::panic::RefUnwindSafe for Foo {}
impl std::marker::Unpin for Foo {}
```

As a workaround, in most cases, you should be able to use [`std::panic::AssertUnwindSafe`](https://doc.rust-lang.org/nightly/std/panic/struct.AssertUnwindSafe.html) instead of implementing one of the `UnwindSafe` traits, and Boxing your type can usually work around the need for `Unpin` (which should be rare in non-`async` code anyway).

### `plrust_suspicious_trait_object`

This lint forbids trait object use in turbofish and generic defaults. This is an effort to fix [certain soundness holes](https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=764d78856996e1985ee88819b013c645) in the Rust language. More simply, the following patterns are disallowed:

```rs
// Trait object in turbofish
foo::<dyn SomeTrait>();
// Trait object in type default (enum, union, trait, and so on are all also forbidden)
struct SomeStruct<T = dyn SomeTrait>(...);
```
