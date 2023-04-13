# Function Arguments

PL/Rust function arguments are mapped in the same order declared by the `CREATE FUNCTION` statement.  They'll have the 
same names and the types will be mapped following the [supported data type mappings](../data-types.md).  Note that the
`STRICT` function property impacts the actual type.  This is described below.

## Naming

The basic rules for naming are:

1. Argument names are case-sensitive.
2. Argument names must also be valid Rust identifiers.  It's best to stick with lowercase ASCII in the set `[a-z0-9_]`.
3. Anonymous argument names are not supported.  Procedural Languages such as `sql` and `plpgsql` support anonymous arguments where they can be referenced as `$1`, `$2`, etc.  PL/Rust does not.

## Argument Ownership

Except in the case of SQL the `TEXT/VARCHAR` and `BYTEA` types, all argument datums are passed to the PL/Rust function
as owned, immutable instances.

## Quick Code Example

Given a `LANGUAGE plrust` function like this:

```sql
CREATE OR REPLACE FUNCTION lots_of_args(a TEXT, b INT, c BOOL[], d JSON) RETURNS INT STRICT LANGUAGE plrust AS $$
   // ... code goes here ...
$$;
```

PL/Rust essentially generates a wrapper Rust function like this:

```rust
use pgx::prelude::*;

fn lots_of_args(a: &str, b: i32, c: Vec<Option<bool>>, d: Json) -> Result<Option<i32>, Box<dyn std::error::Error + Send + Sync + 'static>> {
    // ... code goes here ...
}
```

It is the developer's responsibility to fully implement this function, including [returning the proper value](return-type.md).  
Note that the above is just an abridged example.  The [anatomy](anatomy.md) section describes in detail what really happens.

The section below describes how the `STRICT` keyword impacts the actual function signature, specifically each argument type.

## `STRICT` and `NULL`

PL/Rust uses Rust's `Option<T>` type to represent arguments that might be NULL, plus all return types.  A Postgres UDF
that is not declared as `STRICT` means that any of its arguments *might* be NULL, and PL/Rust is required to account
for this at compile time.  This means that the actual PL/Rust function argument type is context dependent.

As a Postgres refresher, declaring a function as `STRICT` (which is *not* the default) means that if **any** argument
value is `NULL` then the return value is also `NULL`.  In this case, Postgres elides calling the function.

This distinction allows PL/Rust to optimize a bit.  `STRICT` functions have Rust argument types of `T` whereas non-`STRICT`
functions have argument types of `Option<T>`.

Here is the "same" function, the first declared as `STRICT`, the second not:

```sql
CREATE OR REPLACE FUNCTION lcase(s TEXT) RETURNS TEXT STRICT LANGUAGE plrust AS $$
    let lcase = s.to_lowercase();   // `s` is a `&str`
    Ok(Some(lcase)) 
$$;

# SELECT lcase('HELLO WORLD'), lcase(NULL) IS NULL AS is_null;
    lcase    | is_null 
-------------+---------
 hello world | t

```

```sql
CREATE OR REPLACE FUNCTION lcase(s TEXT) RETURNS TEXT LANGUAGE plrust AS $$
    let unwrapped_s = s.unwrap();     // `s` is an `Option<&str>` and will panic if `s` IS NULL
    let lcase = unwrapped_s.to_lowercase();
    Ok(Some(lcase)) 
$$;

# SELECT lcase('HELLO WORLD'), lcase(NULL) IS NULL AS is_null;
ERROR:  called `Option::unwrap()` on a `None` value
```

Rust programmers likely recognize this error message.  When a function is not declared as `STRICT`, it is the programmer's
responsibility to properly handle the possibility of an argument being `Option::None`.

## `STRICT` is an Immutable Property

PL/Rust requires that a `LANGUAGE plrust` function's `STRICT` property be immutable.  As such, PL/Rust prohibits
ALTERing the `STRICT` property:

```sql
ALTER FUNCTION lcase STRICT;
ERROR:  plrust functions cannot have their STRICT property altered
DETAIL:  Use 'CREATE OR REPLACE FUNCTION' to alter the STRICT-ness of an existing plrust function
```

Instead, you must `CREATE OR REPLACE` the function.  The reason for this is that the underlying Rust wrapper function's
signature will be different and this will require that the code be changed to account for the new argument type
(`Option<T>` or `T`).

