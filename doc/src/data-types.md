# Data types

PL/Rust has a rich mapping of database between PostgreSQL data types and
Rust data types. These data type mappings are maintained in the `pgrx` framework
and are [documented in the pgrx documentation](https://github.com/tcdi/pgrx#mapping-of-postgres-types-to-rust).



Documentation of pgrx's `datum` support is on
[docs.rs](https://docs.rs/pgrx/latest/pgrx/datum/index.html).


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


## Mappings through pgrx

Many of the other PostgreSQL data types supported by PL/Rust are implemented
within the `pgrx` framework.


Postgres Type | Rust Type (as `Option<T>`)
--------------|-----------
`json` | `pgrx::Json(serde_json::Value)`
`jsonb` | `pgrx::JsonB(serde_json::Value)`
`date` | `pgrx::Date`
`time` | `pgrx::Time`
`timestamp` | `pgrx::Timestamp`
`time with time zone` | `pgrx::TimeWithTimeZone`
`timestamp with time zone` | `pgrx::TimestampWithTimeZone`
`anyarray` | `pgrx::AnyArray`
`anyelement` | `pgrx::AnyElement`
`box` | `pgrx::pg_sys::BOX`
`point` | `pgrx::pgrx_sys::Point`
`tid` | `pgrx::pg_sys::ItemPointerData`
`cstring` | `&core::ffi::CStr`
`inet` | `pgrx::Inet(String)` -- TODO: needs better support
`numeric` | `pgrx::AnyNumeric` or `pgrx::Numeric<P, S>`
`ARRAY[]::<type>` | `Vec<Option<T>>` or `pgrx::Array<T>` (zero-copy)
`internal` | `pgrx::PgBox<T>` where `T` is any Rust/Postgres struct
`uuid` | `pgrx::Uuid([u8; 16])`


## Specifics


### Numeric support

The `NUMERIC` PostgreSQL data type can map to either
[`pgrx::AnyNumeric`](https://docs.rs/pgrx/latest/pgrx/datum/numeric/struct.AnyNumeric.html)
or
[`pgrx::Numeric<P, S>`](https://docs.rs/pgrx/latest/pgrx/datum/numeric/struct.Numeric.html)
in PL/Rust.

The `pgrx::AnyNumeric` type is the PostgreSQL `NUMERIC` type with default
precision and scale values.  Generally, this is the type you’ll want to use as function arguments when working with numeric data.

The `pgrx::Numeric<P, S>` type is a wrapper around the PostgreSQL
`NUMERIC(P, S)` type. Its Precision and Scale values are known at compile-time to assist with scale conversions and general type safety.

