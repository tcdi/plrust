# Rules and Regulations

This page outlines guidelines for using PL/Rust.

## Database Encoding

PL/Rust only supports databases encoded as `UTF8`.  This is something that must be specified either at `initdb`-time or
during `CREATE DATABASE`.

The reason for this is Rust strictly only supports UTF8-encoding Strings, and requiring this of the database not only
ensures there's no undefined behavior around String conversions, it allows PL/Rust to do certain `TEXT`->`&str` Datum
conversions as zero-copy operations.

## Argument names

PL/Rust functions with arguments require named arguments.
This is different from functions written in
[other languages](https://www.postgresql.org/docs/current/sql-createfunction.html),
such as SQL where `strlen(TEXT, INT)` allows the use of
`$1` and `$2` within the function body.




The succinct reason anonymous parameters are not allowed is because
"It does not align with Rust."  Requiring named parameters
keeps functionality simple, direct and obvious.

One of the reasons people use Rust is because of the quality of the compiler's feedback on incorrect code. Allowing anonymous parameters would ultimately require transforming the code in a way that would either result in potentially garbled error messages, or arbitrarily restricting what sets of identifiers can be used. Simply requiring identifiers skips all of that.

```sql
CREATE OR REPLACE FUNCTION plrust.strlen(TEXT)
    RETURNS BIGINT
    LANGUAGE plrust STRICT
AS $$
    Ok(Some(arg0.len() as i64))
$$;
```


```
ERROR:  PL/Rust does not support unnamed arguments
DETAIL:  PL/Rust argument names must also be valid Rust identifiers.  Rust's identifier specification can be found at https://doc.rust-lang.org/reference/identifiers.html
```


As the above error's detail explains, PL/Rust argument names
must be
[valid Rust identifiers](https://doc.rust-lang.org/reference/identifiers.html).


```sql
CREATE OR REPLACE FUNCTION plrust.strlen("this name is not supported" TEXT)
    RETURNS BIGINT
    LANGUAGE plrust STRICT
AS $$
    Ok(Some("this name is not supported".len() as i64))
$$;
```


## Argument Types


`Option<T>` or `T` depending on `STRICT`


## Return types

`Result<Option<T>>`



