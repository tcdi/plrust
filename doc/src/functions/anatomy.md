# Function Anatomy

A PL/Rust function is Rust code embedded in an SQL `CREATE FUNCTION` statement.  Behind the scenes, PL/Rust injects
the function body into a true Rust function, automatically creating the wrapper function signature along with applying
a set of lints.

It's important to understand the surrounding code environment of an individual `LANGUAGE plrust` function, and this
environment is different depending on certain properties of the function itself.  The important differences arise around
whether the function is declared as `STRICT`.  This is discussed in the [`STRICT` and `NULL`](../data-types.md) chapter.

Using a PL/Rust function that converts a `TEXT` datum to lowercase:

```sql
CREATE OR REPLACE FUNCTION lcase(s TEXT) RETURNS TEXT LANGUAGE plrust AS $$
    Ok(Some(s.unwrap().to_lowercase())) 
$$;
```

PL/Rust then generates the following Rust code:

```rust
mod forbidden {
       #![forbid(deprecated)]
       #![forbid(implied_bounds_entailment)]
       #![forbid(plrust_async)]
       #![forbid(plrust_autotrait_impls)]
       #![forbid(plrust_env_macros)]
       #![forbid(plrust_extern_blocks)]
       #![forbid(plrust_external_mod)]
       #![forbid(plrust_filesystem_macros)]
       #![forbid(plrust_fn_pointers)]
       #![forbid(plrust_leaky)]
       #![forbid(plrust_lifetime_parameterized_traits)]
       #![forbid(plrust_print_macros)]
       #![forbid(plrust_static_impls)]
       #![forbid(plrust_stdio)]
       #![forbid(plrust_suspicious_trait_object)]
       #![forbid(soft_unstable)]
       #![forbid(suspicious_auto_trait_impls)]
       #![forbid(unaligned_references)]
       #![forbid(unsafe_code)]
       #![forbid(where_clauses_object_safety)]
    
       #[allow(unused_imports)]
       use pgx::prelude::*;
    
       #[allow(unused_lifetimes)]
       fn plrust_fn_oid_16384_16404<'a>(
           s: Option<&'a str>,
       ) -> ::std::result::Result<
           Option<String>,
           Box<dyn std::error::Error + Send + Sync + 'static>,
       > {
           Ok(Some(s.unwrap().to_lowercase()))
       }
   }
```

### `mod forbidden {}`

Every PL/Rust function is wrapped with this module and cannot be influenced by users.  It exists so that PL/Rust can 
apply [lints](../config-lints.md) to the user's code which will detect forbidden code patterns and idioms at compile time.

### `#[!forbid(...)]`

These are the lints that, if triggered, will fail compilation.  These [lints](../config-lints.md) are only applied here 
and are not applied to external dependencies.


### `use pgx::prelude::*`

A default set of types and traits available to every PL/Rust function.  Despite the name, these originate from 
[`plrust-trusted-pgx`](https://docs.rs/plrust-trusted-pgx/latest/plrust_trusted_pgx/).  `plrust-trusted-pgx` is a very
small subset of `pgx`, the crate upon which PL/Rust *and* `LANGUAGE plrust` functions are based.

### `fn plrust_fn_oid_16384_16404(...) -> ... {}`

The function in which the `LANGUAGE plrust` function body is injected.  The naming convention is the literal string
`plrust_fn_oid_`, then the database's `OID`, an underscore, and the function's `OID`.

A PL/Rust function author does not need to know this function name and would never have a reason to call it directly, but 
it's important to know how the name is generated.

Generation of the function's [arguments](arguments.md) and [return type](return-type.md) are discussed in more detail in 
their respective sections.


### `Ok(Some(s.unwrap().to_lowercase()))`

The function body itself.  This is injected, unchanged, directly from the body of the `CREATE FUNCTION` statement.

It's worth nothing that the function body is parsed for syntactic correctness by the Rust crate `syn` prior to 
generating the entire block of code outlined here.  This means PL/Rust doesn't rely on the compiler for syntax checking 
-- it happens up-front.  As such, syntax errors may report error messages that are sometimes unhelpful.

