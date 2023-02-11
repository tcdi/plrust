# Built-in functions

This page documents many of the high level functions,
targeted functionality is covered on dedicated sub-sections.

- [Server Programming Interface (SPI)](spi.md)
- [Triggers](triggers.md)
- [Additional features](additional-features.md)


## Functions available

Functions available to PL/Rust are defined under
the [`trusted-pgx` directory in `lib.rs`](https://github.com/tcdi/plrust/blob/main/trusted-pgx/src/lib.rs). User functions in `plrust` will not compile if they use
the `unsafe` keyword.
There are a handful of functions in `trusted-pgx` that are
declared unsafe; `plrust` functions cannot use them because they would need an `unsafe {}` block.


## Datum functions

PL/Rust function support for various Datums are documented by
[pgx on docs.rs](https://docs.rs/pgx/latest/pgx/datum/index.html),
the source is [on GitHub](https://github.com/tcdi/pgx/tree/master/pgx/src/datum) for those interested.
There are Datums defined in `pgx` that are not included in PL/Rust
because they have not been imported by `plrust`.


[`AnyNumeric`](https://docs.rs/pgx/latest/pgx/datum/numeric/struct.AnyNumeric.html):
A plain PostgreSQL `NUMERIC` with default precision and scale values. This is a sufficient type to represent any Rust primitive value from `i128::MIN` to `u128::MAX` and anything in between.

[`FromDatum`](https://docs.rs/pgx/latest/pgx/datum/trait.FromDatum.html) and [`IntoDatum`](https://docs.rs/pgx/latest/pgx/datum/trait.IntoDatum.html): Provide conversions between `pg_sys::Datum` and Rust types. 


[`Json`](https://docs.rs/pgx/latest/pgx/datum/struct.Json.html)
and
[`JsonB`](https://docs.rs/pgx/latest/pgx/datum/struct.JsonB.html)
match the types in PostgreSQL of the same name.


[`Date`](https://docs.rs/pgx/latest/pgx/datum/struct.Date.html):
A plain PostgreSQL `DATE` type without a time component.


`Time` / `TimeWithTimeZone` / `Timestamp` / `TimestampWithTimeZone`




## fcinfo functions

`pg_getarg`

`pg_return_null`

`pg_return_void`

`srf_first_call_init`

`srf_is_first_call`

`srf_per_call_setup`

`srf_return_done`

`srf_return_next`




