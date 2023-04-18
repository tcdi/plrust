# Built-in functions

This page documents many of the high level functions,
targeted functionality is covered on dedicated sub-sections.

- [Server Programming Interface (SPI)](spi.md)
- [Triggers](triggers.md)
- [Logging to PostgreSQL from PL/Rust](logging.md)


## Functions available

Functions available to PL/Rust are defined under
the [`trusted-pgrx` directory in `lib.rs`](https://github.com/tcdi/plrust/blob/main/trusted-pgrx/src/lib.rs). User functions in `plrust` will not compile if they use
the `unsafe` keyword.
There are a handful of functions in `trusted-pgrx` that are
declared unsafe; `plrust` functions cannot use them because they would need an `unsafe {}` block.


## Datum functions

PL/Rust function support for various Datums are documented by
[pgrx on docs.rs](https://docs.rs/pgrx/latest/pgrx/datum/index.html),
the source is [on GitHub](https://github.com/tcdi/pgrx/tree/master/pgrx/src/datum) for those interested.
There are Datums defined in `pgrx` that are not included in PL/Rust
because they have not been imported by `plrust`.


[`AnyNumeric`](https://docs.rs/pgrx/latest/pgrx/datum/numeric/struct.AnyNumeric.html):
A plain PostgreSQL `NUMERIC` with default precision and scale values. This is a sufficient type to represent any Rust primitive value from `i128::MIN` to `u128::MAX` and anything in between.

[`FromDatum`](https://docs.rs/pgrx/latest/pgrx/datum/trait.FromDatum.html) and [`IntoDatum`](https://docs.rs/pgrx/latest/pgrx/datum/trait.IntoDatum.html): Provide conversions between `pg_sys::Datum` and Rust types. 


[`Json`](https://docs.rs/pgrx/latest/pgrx/datum/struct.Json.html)
and
[`JsonB`](https://docs.rs/pgrx/latest/pgrx/datum/struct.JsonB.html)
match the types in PostgreSQL of the same name.


[`Date`](https://docs.rs/pgrx/latest/pgrx/datum/struct.Date.html):
A plain PostgreSQL `DATE` type without a time component.


`Time` / `TimeWithTimeZone` / `Timestamp` / `TimestampWithTimeZone`


Range Support In progress


