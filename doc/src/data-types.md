# Data types

PL/Rust provides mappings for many of the built-in PostgreSQL data types.  Rust's ownership rules means that these
mappings may be different based on their usage.  Where it can PL/Rust borrows (zero-copy) arguments and returns
owned values.

| SQL                | PL/Rust Argument           | PL/Rust Return Type        |
|--------------------|----------------------------|----------------------------|
| `bytea`            | `&[u8]`                    | `Vec<u8>`                  |
| `text`             | `&str`                     | `String`                   |
| `varchar`          | `&str`                     | `String`                   |
| `json`             | `Json(serde_json::Value)`  | `Json(serde_json::Value)`  |
| `jsonb`            | `JsonB(serde_json::Value)` | `JsonB(serde_json::Value)` |
| `box`              | `BOX`<sup>1</sup>          | `BOX`                      |
| `point`            | `Point`                    | `Point`                    |
| `cstring`          | `&CStr`                    | `CString`                  |
| `oid`              | `Oid`                      | `Oid`                      |
| `tid`              | `ItemPointerData`          | `ItemPointerData`          |
| `uuid`             | `Uuid`                     | `Uuid`                     |
| `int4range`        | `Range<i32>`               | `Range<i32>`               |
| `int8range`        | `Range<i64>`               | `Range<i64>`               |
| `numrange`         | `Range<AnyNumeric>`        | `Range<AnyNumeric>`        |
| `"char"`           | `i8`                       | `i8`                       |
| `smallint`         | `i16`                      | `i16`                      |
| `integer`          | `i32`                      | `i32`                      |
| `bigint`           | `i64`                      | `i64`                      |
| `real`             | `f32`                      | `f32`                      |
| `double precision` | `f64`                      | `f64`                      |
| `numeric`          | `AnyNumeric`               | `AnyNumeric`               |
| `bool`             | `bool`                     | `bool`                     |
| `void`             | n/a                        | `()`                       |
| `NULL`             | `Option::None`             | `Option::None`             |

<sup>1: This is Postgres' geometric BOX type, not to be confused with Rust's `Box` type, which stores allocated data on the heap</sup>


## Date and Time Support?

PL/Rust does not currently support PostgreSQL's various "date" and "time" types.  Support will be added in a future
release.

