# Data types

PL/Rust has a rich mapping of database between PostgreSQL data types and
Rust data types. These data type mappings are maintained in the `pgx` framework
and are [documented in the pgx documentation](https://github.com/tcdi/pgx#mapping-of-postgres-types-to-rust).



Documentation of pgx's `datum` support is on
[docs.rs](https://docs.rs/pgx/latest/pgx/datum/index.html).


## Direct mappings

A few of PostgreSQL's data types map directly to Rust data types. For example,
each of the variations of `INTEGER` has a direct mapping to a Rust type,
e.g. `SMALLINT` -> `i8` and `BIGINT` -> `i64`.

Postgres Type | Rust Type (as `Option<T>`)
--------------|-----------
`bytea` | `Vec<u8>` or `&[u8]` (zero-copy)
`text` | `String` or `&str` (zero-copy)
`varchar` | `String` or `&str` (zero-copy) or `char`
`"char"` | `i8`
`smallint` | `i16`
`integer` | `i32`
`bigint` | `i64`
`oid` | `u32`
`real` | `f32`
`double precision` | `f64`
`bool` | `bool`
`void`  | `()`
`NULL` | `Option::None`


## Mappings through pgx

Many of the other PostgreSQL data types supported by PL/Rust are implemented
within the `pgx` framework.


Postgres Type | Rust Type (as `Option<T>`)
--------------|-----------
`json` | `pgx::Json(serde_json::Value)`
`jsonb` | `pgx::JsonB(serde_json::Value)`
`date` | `pgx::Date`
`time` | `pgx::Time`
`timestamp` | `pgx::Timestamp`
`time with time zone` | `pgx::TimeWithTimeZone`
`timestamp with time zone` | `pgx::TimestampWithTimeZone`
`anyarray` | `pgx::AnyArray`
`anyelement` | `pgx::AnyElement`
`box` | `pgx::pg_sys::BOX`
`point` | `pgx::pgx_sys::Point`
`tid` | `pgx::pg_sys::ItemPointerData`
`cstring` | `&core::ffi::CStr`
`inet` | `pgx::Inet(String)` -- TODO: needs better support
`numeric` | `pgx::AnyNumeric` or `pgx::Numeric<P, S>`
`ARRAY[]::<type>` | `Vec<Option<T>>` or `pgx::Array<T>` (zero-copy)
`internal` | `pgx::PgBox<T>` where `T` is any Rust/Postgres struct
`uuid` | `pgx::Uuid([u8; 16])`


## Specifics


### Numeric support

The `NUMERIC` PostgreSQL data type can map to either
[`pgx::AnyNumeric`](https://docs.rs/pgx/latest/pgx/datum/numeric/struct.AnyNumeric.html)
or
[`pgx::Numeric<P, S>`](https://docs.rs/pgx/latest/pgx/datum/numeric/struct.Numeric.html)
in PL/Rust.

The `pgx::AnyNumeric` type is the PostgreSQL `NUMERIC` type with default
precision and scale values.  Generally, this is the type you’ll want to use as function arguments when working with numeric data.

The `pgx::Numeric<P, S>` type is a wrapper around the PostgreSQL
`NUMERIC(P, S)` type. Its Precision and Scale values are known at compile-time to assist with scale conversions and general type safety.

