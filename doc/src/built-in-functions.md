# Built-in functions


Most of what is found here can be used.

https://github.com/tcdi/plrust/blob/main/trusted-pgx/src/lib.rs


`plrust` user functions won't compile if they use the `unsafe` keyword.
There's a handful of functions in `trusted-pgx` that are declared unsafe,
so `plrust` functions can not use them because they would need an `unsafe {}`
block.



