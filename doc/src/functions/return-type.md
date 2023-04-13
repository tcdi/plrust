# Return Type

Every `LANUAGE plrust` function has the same general return type, which is:

```rust
fn foo() -> Result<
        Option<T>, 
        Box<dyn std::error::Error + Send + Sync + 'static>
> {
    
}
```

The `T` is determined by the mapping from the declared SQL type during `CREATE FUNCTION`, and the rest is essentially
boilerplate to allow easy handling of Rust `Result`s and the SQL concept of NULL.

## Why `Option<T>`?

Any PostgreSQL procedural language function can return NULL.  PL/Rust understands and represents SQL NULL as `Option::None`.
It may seem cumbersome, but PL/Rust function must return an `Option<T>`, as either `Some(T)` (non-null value) or `None` (NULL value).

While PostgreSQL's `STRICT` function property can influence the return value such that "any NULL argument guarantees a 
NULL return", Postgres does not have a way to express that a function "will never return NULL".  As such, PL/Rust 
functions have the opportunity to return NULL built into their underlying function signature.

If a PL/Rust function would never return NULL, always return the `Some` variant.

## Why `Result<..., Box<dyn std::error::Error + Send + Sync + 'static>>`?

Generally speaking, Postgres procedural language functions, and even Postgres internals, can be considered "fail fast"
in that they tend to raise an error/exception at the exact point when it happens.  Rust tends towards propagating errors
up the stack, relying on the caller to handle it.  

PL/Rust bridges this gap by requiring all `LANGUAGE plrust` functions to return a `Result`, and PL/Rust itself will 
interpret the return value from the function and report a `Result::Err(e)` as a Postgres `ERROR`, aborting the current 
transaction.

Returning a `Result` helps to simplify error handling, especially when a `LANGUAGE plrust` function uses [Spi](../spi.md)
as the Rust `?` operator is usable to propagate errors during function execution.

Since the  Rust "Error" type cannot be expressed as part of the `CREATE FUNCTION` statement, PL/Rust generalizes the 
error to `Box<dyn std::error::Error + Send + Sync + 'static>` to provide as much compatability as possible with the
wide range of concrete Error types in the Rust ecosystem.





