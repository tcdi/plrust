# No Unsigned Types

Rust programmers may be asking "where are the unsigned types like `u32`?".  PostgreSQL does not have unsigned integer types.
As such, neither does PL/Rust.  

In order to represent a value larger than `i32::MAX`, `BIGINT` is the proper SQL type.  To represent a value larger than 
`i64::MAX`, use the `NUMERIC` type.  Postgres also has no concept of an `isize` or `usize`, so these have no 
corresponding SQL mapping.

PL/Rust's `AnyNumeric` type has `From` and `TryFrom` implementations for all of Rust's primitive types (plus strings).
This makes it fairly straightforward to up-convert a Rust primitive into a SQL `NUMERIC`:

```sql
CREATE OR REPLACE FUNCTION upconvert_bigint(i BIGINT) RETURNS NUMERIC STRICT LANGUAGE plrust AS $$
    let n: AnyNumeric = i.into();   // `i` is an `i64`, lets convert to `AnyNumeric`
    Ok(Some(n + 1))    
$$;

# SELECT upconvert_bigint(9223372036854775807);
upconvert_bigint   
---------------------
 9223372036854775808
(1 row)
```
