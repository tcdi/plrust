# Built-in functions


Most of what is found here can be used.

https://github.com/tcdi/plrust/blob/main/trusted-pgx/src/lib.rs

## Note - Cleanup

`plrust` user functions won't compile if they use the `unsafe` keyword.
There's a handful of functions in `trusted-pgx` that are declared unsafe,
so `plrust` functions can not use them because they would need an `unsafe {}`
block.


## Datum functions

`AnyNumeric`

`Date`

`FromDatum` / `IntoDatum`

`Json` / `JsonB`

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




