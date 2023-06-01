# Data types

PL/Rust provides mappings for many of the built-in PostgreSQL data types.  Rust's ownership rules means that these
mappings may be different based on their usage.  Where it can PL/Rust borrows (zero-copy) arguments and returns
owned values.

| SQL                        | PL/Rust Argument               | PL/Rust Return Type            |
|----------------------------|--------------------------------|--------------------------------|
| `NULL`                     | `Option::None`                 | `Option::None`                 |
| `"char"`                   | `i8`                           | `i8`                           |
| `bigint`                   | `i64`                          | `i64`                          |
| `bool`                     | `bool`                         | `bool`                         |
| `box`                      | `BOX`<sup>1</sup>              | `BOX`                          |
| `bytea`                    | `&[u8]`                        | `Vec<u8>`                      |
| `cstring`                  | `&CStr`                        | `CString`                      |
| `date`                     | `Date`                         | `Date`                         |
| `daterange`                | `Range<Date>`                  | `Range<Date>`                  |
| `double precision`         | `f64`                          | `f64`                          |
| `int4range`                | `Range<i32>`                   | `Range<i32>`                   |
| `int8range`                | `Range<i64>`                   | `Range<i64>`                   |
| `integer`                  | `i32`                          | `i32`                          |
| `interval`                 | `Interval`                     | `Interval`                     |
| `json`                     | `Json(serde_json::Value)`      | `Json(serde_json::Value)`      |
| `jsonb`                    | `JsonB(serde_json::Value)`     | `JsonB(serde_json::Value)`     |
| `numeric`                  | `AnyNumeric`                   | `AnyNumeric`                   |
| `numrange`                 | `Range<AnyNumeric>`            | `Range<AnyNumeric>`            |
| `oid`                      | `Oid`                          | `Oid`                          |
| `point`                    | `Point`                        | `Point`                        |
| `real`                     | `f32`                          | `f32`                          |
| `smallint`                 | `i16`                          | `i16`                          |
| `text`                     | `&str`                         | `String`                       |
| `tid`                      | `ItemPointerData`              | `ItemPointerData`              |
| `time with time zone`      | `TimeWithTimeZone`             | `TimeWithTimeZone`             |
| `time`                     | `Time`                         | `Time`                         |
| `timestamp with time zone` | `TimestampWithTimeZone`        | `TimestampWithTimeZone`        |
| `timestamp`                | `Timestamp`                    | `Timestamp`                    |
| `tsrange`                  | `Range<Timestamp>`             | `Range<Timestamp>`             |
| `tstzrange`                | `Range<TimestampWithTimeZone>` | `Range<TimestampWithTimeZone>` |
| `uuid`                     | `Uuid`                         | `Uuid`                         |
| `varchar`                  | `&str`                         | `String`                       |
| `void`                     | n/a                            | `()`                           |

<sup>1: This is Postgres' geometric BOX type, not to be confused with Rust's `Box` type, which stores allocated data on the heap</sup>
